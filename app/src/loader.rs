//! Background media loading — worker pool, preview-first, single decode per file.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::{self, JoinHandle};

use cap_core::{classify_extension, MediaKind};
use cap_image::{
    decode_full, decode_gif_animation, decode_prefetch, decode_staged, AnimFrame, DecodedImage,
    PREFETCH_EDGE, PREVIEW_EDGE,
};
use cap_model::{load_mesh, MeshData, ModelInfo};
use cap_viewer::OrbitCamera;
use crossbeam_channel::{Receiver, Sender, TryRecvError, unbounded};

const MEDIA_WORKERS: usize = 2;

static GENERATION: AtomicU64 = AtomicU64::new(1);

pub fn next_generation() -> u64 {
    GENERATION.fetch_add(1, Ordering::SeqCst)
}

pub fn normalize_path(path: PathBuf) -> PathBuf {
    path.canonicalize().unwrap_or(path)
}

pub fn paths_equal(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    #[cfg(windows)]
    {
        a.to_string_lossy()
            .eq_ignore_ascii_case(&b.to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Result of a background load job.
pub enum LoadedPayload {
    Image {
        decoded: DecodedImage,
        animation: Vec<AnimFrame>,
    },
    Model {
        info: ModelInfo,
        mesh: Option<std::sync::Arc<MeshData>>,
        camera: OrbitCamera,
    },
}

pub enum LoadMessage {
    /// Fast low-res preview while full image decodes.
    Preview {
        path: PathBuf,
        generation: u64,
        decoded: DecodedImage,
    },
    Ready {
        path: PathBuf,
        generation: u64,
        result: Result<LoadedPayload, String>,
    },
    Thumbnail {
        path: PathBuf,
        /// `None` means decode failed — UI should stop retrying.
        decoded: Option<DecodedImage>,
    },
}

enum MediaJob {
    Load { path: PathBuf, generation: u64 },
    Prefetch { path: PathBuf },
    FullRes { path: PathBuf, generation: u64 },
}

enum ThumbJob {
    Decode(PathBuf),
}

/// Bounded worker pool for media loads; thumbs on dedicated threads.
pub struct MediaLoader {
    media_tx: Sender<MediaJob>,
    msg_rx: Receiver<LoadMessage>,
    thumb_tx: Sender<ThumbJob>,
    _media_workers: Vec<JoinHandle<()>>,
    _thumb_workers: Vec<JoinHandle<()>>,
}

impl MediaLoader {
    pub fn spawn() -> Self {
        let (msg_tx, msg_rx) = unbounded::<LoadMessage>();
        let (media_tx, media_rx) = unbounded::<MediaJob>();
        let (thumb_tx, thumb_rx) = unbounded::<ThumbJob>();

        let mut media_workers = Vec::with_capacity(MEDIA_WORKERS);
        for i in 0..MEDIA_WORKERS {
            let rx = media_rx.clone();
            let tx = msg_tx.clone();
            media_workers.push(
                thread::Builder::new()
                    .name(format!("lookeveryting-media-{i}"))
                    .spawn(move || media_worker_loop(rx, tx))
                    .expect("spawn media thread"),
            );
        }

        const THUMB_WORKERS: usize = 3;
        let mut thumb_workers = Vec::with_capacity(THUMB_WORKERS);
        for i in 0..THUMB_WORKERS {
            let rx = thumb_rx.clone();
            let tx = msg_tx.clone();
            thumb_workers.push(
                thread::Builder::new()
                    .name(format!("lookeveryting-thumbs-{i}"))
                    .spawn(move || thumb_loop(rx, tx))
                    .expect("spawn thumb thread"),
            );
        }

        Self {
            media_tx,
            msg_rx,
            thumb_tx,
            _media_workers: media_workers,
            _thumb_workers: thumb_workers,
        }
    }

    pub fn request_media(&self, path: PathBuf, generation: u64) {
        let _ = self
            .media_tx
            .send(MediaJob::Load { path, generation });
    }

    pub fn request_prefetch(&self, path: PathBuf) {
        let _ = self.media_tx.send(MediaJob::Prefetch { path });
    }

    pub fn request_full_res(&self, path: PathBuf, generation: u64) {
        let _ = self
            .media_tx
            .send(MediaJob::FullRes { path, generation });
    }

    pub fn request_thumbnail(&self, path: PathBuf) {
        let _ = self.thumb_tx.send(ThumbJob::Decode(path));
    }

    pub fn poll(&self) -> Vec<LoadMessage> {
        let mut out = Vec::new();
        loop {
            match self.msg_rx.try_recv() {
                Ok(msg) => out.push(msg),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        out
    }
}

fn media_worker_loop(jobs: Receiver<MediaJob>, tx: Sender<LoadMessage>) {
    while let Ok(job) = jobs.recv() {
        match job {
            MediaJob::Load { path, generation } => load_media_job(path, generation, &tx),
            MediaJob::Prefetch { path } => load_prefetch_job(path, &tx),
            MediaJob::FullRes { path, generation } => load_full_res_job(path, generation, &tx),
        }
    }
}

fn load_media_job(path: PathBuf, generation: u64, tx: &Sender<LoadMessage>) {
    if classify_extension(&path) == Some(MediaKind::Image) {
        let animation = decode_gif_animation(&path).unwrap_or_default();
        match decode_staged(&path, PREVIEW_EDGE, cap_image::MAX_VIEW_EDGE) {
            Ok((Some(preview), view)) => {
                let _ = tx.send(LoadMessage::Preview {
                    path: path.clone(),
                    generation,
                    decoded: preview,
                });
                let _ = tx.send(LoadMessage::Ready {
                    path,
                    generation,
                    result: Ok(LoadedPayload::Image {
                        decoded: view,
                        animation,
                    }),
                });
            }
            Ok((None, view)) => {
                let _ = tx.send(LoadMessage::Ready {
                    path,
                    generation,
                    result: Ok(LoadedPayload::Image {
                        decoded: view,
                        animation,
                    }),
                });
            }
            Err(err) => {
                let _ = tx.send(LoadMessage::Ready {
                    path,
                    generation,
                    result: Err(err.to_string()),
                });
            }
        }
        return;
    }

    let result = load_media_sync(&path);
    let _ = tx.send(LoadMessage::Ready {
        path,
        generation,
        result,
    });
}

fn load_full_res_job(path: PathBuf, generation: u64, tx: &Sender<LoadMessage>) {
    let result = decode_full(&path)
        .map(|decoded| LoadedPayload::Image {
            decoded,
            animation: Vec::new(),
        })
        .map_err(|e| e.to_string());
    let _ = tx.send(LoadMessage::Ready {
        path,
        generation,
        result,
    });
}

fn load_prefetch_job(path: PathBuf, tx: &Sender<LoadMessage>) {
    if classify_extension(&path) != Some(MediaKind::Image) {
        return;
    }
    if let Ok(decoded) = decode_prefetch(&path, PREFETCH_EDGE) {
        let _ = tx.send(LoadMessage::Ready {
            path,
            generation: u64::MAX,
            result: Ok(LoadedPayload::Image {
                decoded,
                animation: Vec::new(),
            }),
        });
    }
}

fn thumb_loop(jobs: Receiver<ThumbJob>, out: Sender<LoadMessage>) {
    while let Ok(ThumbJob::Decode(path)) = jobs.recv() {
        let decoded = match classify_extension(&path) {
            Some(MediaKind::Image) => {
                cap_image::decode_thumbnail(&path, 192).ok()
            }
            Some(MediaKind::Video) => cap_video::decode_thumbnail(&path, 192).map(|frame| {
                DecodedImage {
                    width: frame.width,
                    height: frame.height,
                    rgba: frame.rgba,
                    native_width: frame.width,
                    native_height: frame.height,
                }
            }),
            Some(MediaKind::Model) => load_mesh(&path).ok().and_then(|mesh| {
                cap_viewer::render_mesh_thumbnail(&mesh, 192).map(|rgba| DecodedImage {
                    width: 192,
                    height: 192,
                    rgba,
                    native_width: 192,
                    native_height: 192,
                })
            }),
            _ => None,
        };
        let _ = out.send(LoadMessage::Thumbnail { path, decoded });
    }
}

fn load_media_sync(path: &Path) -> Result<LoadedPayload, String> {
    match classify_extension(path) {
        Some(MediaKind::Video) => Err("video uses UI thread".into()),
        Some(MediaKind::Model) => {
            let info = ModelInfo::from_path(path).map_err(|e| e.to_string())?;
            let mesh = load_mesh(path).ok().map(std::sync::Arc::new);
            let camera = mesh
                .as_ref()
                .map(|m| OrbitCamera::fit_bounds(&m.bounds))
                .unwrap_or_default();
            Ok(LoadedPayload::Model {
                info,
                mesh,
                camera,
            })
        }
        _ => Err("unsupported file format".into()),
    }
}

/// LRU cache for decoded images.
pub struct ImageCache {
    entries: HashMap<PathBuf, DecodedImage>,
    order: Vec<PathBuf>,
    capacity: usize,
}

impl ImageCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: HashMap::new(),
            order: Vec::new(),
            capacity: capacity.max(1),
        }
    }

    pub fn get(&self, path: &Path) -> Option<&DecodedImage> {
        self.entries.get(path)
    }

    pub fn insert(&mut self, path: PathBuf, decoded: DecodedImage) -> Vec<PathBuf> {
        let mut evicted = Vec::new();
        if self.entries.contains_key(&path) {
            self.order.retain(|p| p != &path);
        }
        self.order.push(path.clone());
        self.entries.insert(path, decoded);
        while self.order.len() > self.capacity {
            if let Some(old) = self.order.first().cloned() {
                self.order.remove(0);
                self.entries.remove(&old);
                evicted.push(old);
            }
        }
        evicted
    }
}

impl Default for ImageCache {
    fn default() -> Self {
        Self::new(12)
    }
}
