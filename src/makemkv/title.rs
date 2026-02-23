#[derive(Debug, Clone)]
pub struct TitleList {
    handle: u64,
    size: u32,
    titles: Vec<Option<Title>>,
}

impl TitleList {
    pub fn new(handle: u64, size: u32) -> Self {
        Self {
            handle,
            titles: vec![None; size as usize],
            size,
        }
    }

    pub fn add_title(
        &mut self,
        index: u32,
        handle: u64,
        chapter_handle: u64,
        chapter_size: u32,
        track_size: u32,
    ) {
        self.titles[index as usize] =
            Some(Title::new(handle, chapter_handle, chapter_size, track_size))
    }
}

#[derive(Debug, Clone)]
pub struct Title {
    chapters: ChapterList,
    tracks: Vec<Option<Track>>,
    size: u32,
    handle: u64,
}

impl Title {
    pub fn new(handle: u64, chapter_handle: u64, chapter_size: u32, track_size: u32) -> Self {
        Self {
            chapters: ChapterList::new(chapter_handle, chapter_size),
            tracks: vec![None; track_size as usize],
            handle,
            size: track_size,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Track {}

#[derive(Debug, Clone)]
pub struct ChapterList {
    handle: u64,
    size: u32,
    chapters: Vec<Option<Chapter>>,
}

impl ChapterList {
    pub fn new(handle: u64, size: u32) -> Self {
        Self {
            handle,
            chapters: vec![None; size as usize],
            size,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Chapter {}
