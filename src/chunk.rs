use anyhow::anyhow;
use fundsp::prelude::lerp;
use std::f32;
use std::sync::Arc;

pub const CHUNK_SAMPLES: usize = 44_100 * 60;
const BLEND_WINDOW: usize = 1000;

pub type Samples = [f32; CHUNK_SAMPLES];

#[derive(Debug, Clone)]
pub struct PlayableChunk {
    data: Arc<Samples>,
}

impl PlayableChunk {
    pub fn new(data: Vec<f32>) -> Result<Self, anyhow::Error> {
        if data.len() != CHUNK_SAMPLES {
            return Err(anyhow!("Length mismatch"));
        }

        let arc: Arc<[f32]> = Arc::from(data);
        let ptr = Arc::into_raw(arc) as *mut [f32; CHUNK_SAMPLES];
        let data = unsafe { Arc::from_raw(ptr) };
        Ok(Self { data })
    }

    pub fn get(&self, idx: usize) -> Option<&f32> {
        self.data.get(idx)
    }
}

impl IntoIterator for PlayableChunk {
    type Item = f32;

    type IntoIter = PlayableChunkIter;

    fn into_iter(self) -> Self::IntoIter {
        PlayableChunkIter {
            data: self.data,
            fwd_idx: 0,
            bwd_idx: CHUNK_SAMPLES,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlayableChunkIter {
    data: Arc<Samples>,
    fwd_idx: usize,
    bwd_idx: usize,
}

impl Iterator for PlayableChunkIter {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.fwd_idx < self.bwd_idx {
            let res = self.data[self.fwd_idx];
            self.fwd_idx += 1;
            Some(res)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = CHUNK_SAMPLES - self.fwd_idx;
        (remaining, Some(remaining))
    }
}

impl DoubleEndedIterator for PlayableChunkIter {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.fwd_idx < self.bwd_idx {
            self.bwd_idx -= 1;
            let res = self.data[self.bwd_idx];
            Some(res)
        } else {
            None
        }
    }
}

impl ExactSizeIterator for PlayableChunkIter {}

#[derive(Debug, Clone, Copy)]
pub enum SmoothingType {
    Mirror,
    Blend(BlendType),
}

impl Default for SmoothingType {
    fn default() -> Self {
        Self::Mirror
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BlendType {
    Linear,
    Sigmoid,
}

impl BlendType {
    fn blend(self, a: f32, b: f32, t: f32) -> f32 {
        let weight = match self {
            BlendType::Linear => t,
            BlendType::Sigmoid => {
                // sigmoid (logistical) function, converges fast between -6 and 6
                let sig = |x: f32| 1.0 / (1.0 + f32::powf(f32::consts::E, -x));
                let scaled_t = 12.0 * t - 6.0;
                sig(scaled_t)
            }
        };

        lerp(a, b, weight)
    }
}

pub struct ChunkCollection {
    chunks: Vec<PlayableChunk>,
    smoothing_type: SmoothingType,
}

type SampleIter = Box<dyn Iterator<Item = (f32, f32)> + Send>;

struct BlendingChunksIter<I, J> {
    chunk_iter: I,
    current_chunk: J,
    next_chunk: J,
    blend_type: BlendType,
}

impl ChunkCollection {
    pub fn new(chunks: Vec<PlayableChunk>) -> Result<Self, anyhow::Error> {
        if chunks.is_empty() {
            return Err(anyhow!("Empty chunks"));
        }
        Ok(Self {
            chunks,
            smoothing_type: Default::default(),
        })
    }

    pub fn with_mirror(mut self) -> Self {
        self.smoothing_type = SmoothingType::Mirror;
        self
    }

    pub fn with_blend(mut self, blend_type: BlendType) -> Self {
        self.smoothing_type = SmoothingType::Blend(blend_type);
        self
    }

    pub fn into_iter(self) -> Result<SampleIter, anyhow::Error> {
        match self.smoothing_type {
            SmoothingType::Mirror => {
                let first_chunk = self.chunks.into_iter().next().unwrap();

                let iter = first_chunk
                    .clone()
                    .into_iter()
                    .chain(first_chunk.into_iter().rev())
                    .map(|f| (f, f))
                    .cycle();

                Ok(Box::new(iter))
            }
            SmoothingType::Blend(blend_type) => Ok(Box::new(BlendingChunksIter::new(
                self.chunks.into_iter(),
                blend_type,
            )?)),
        }
    }
}

impl<I: Iterator<Item = J>, J: IntoIterator<IntoIter = K>, K> BlendingChunksIter<I, K> {
    fn new(mut chunk_iter: I, blend_type: BlendType) -> Result<Self, anyhow::Error> {
        let current_chunk = chunk_iter.next().unwrap();
        let next_chunk = chunk_iter.next().unwrap();

        Ok(Self {
            chunk_iter: chunk_iter,
            blend_type,
            current_chunk: current_chunk.into_iter(),
            next_chunk: next_chunk.into_iter(),
        })
    }
}

impl<
        I: Iterator<Item = J>,
        J: IntoIterator<Item = f32, IntoIter = K>,
        K: Iterator<Item = f32> + ExactSizeIterator,
    > Iterator for BlendingChunksIter<I, K>
{
    type Item = (f32, f32);

    fn next(&mut self) -> Option<Self::Item> {
        let s = if self.current_chunk.len() <= BLEND_WINDOW {
            let weight = 1.0 - self.current_chunk.len() as f32 / BLEND_WINDOW as f32;
            let s1 = self.current_chunk.next().unwrap();
            let s2 = self.next_chunk.next().unwrap();

            let s = self.blend_type.blend(s1, s2, weight);

            if self.current_chunk.len() == 0 {
                let new_next = self.chunk_iter.next().unwrap().into_iter();
                let old_next = std::mem::replace(&mut self.next_chunk, new_next);
                self.current_chunk = old_next;
            }

            s
        } else {
            self.current_chunk.next().unwrap()
        };

        Some((s, s))
    }
}
