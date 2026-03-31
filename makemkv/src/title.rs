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

    pub fn get_title(&mut self, index: u32) -> Option<&Title> {
        if index >= self.size {
            return None;
        }
        self.titles[index as usize].as_ref()
    }

    pub fn add_track(&mut self, title_index: u32, track_index: u32, handle: u64) {
        if title_index >= self.size {
            return;
        }

        if let Some(title) = &mut self.titles[title_index as usize] {
            title.add_track(track_index, handle);
        }
    }

    pub fn add_chapter(&mut self, title_index: u32, chapter_index: u32, handle: u64) {
        if title_index >= self.size {
            return;
        }

        if let Some(title) = &mut self.titles[title_index as usize] {
            title.chapters.add_chapter(chapter_index, handle);
        }
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

    pub fn add_track(&mut self, index: u32, handle: u64) {
        if index < self.size {
            self.tracks[index as usize] = Some(Track::new(handle));
        }
    }
}

#[derive(Debug, Clone)]
pub struct Track {
    handle: u64,
}

impl Track {
    pub fn new(handle: u64) -> Self {
        Self { handle }
    }
}

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

    pub fn add_chapter(&mut self, index: u32, handle: u64) {
        if index >= self.size {
            return;
        }

        self.chapters[index as usize] = Some(Chapter::new(handle));
    }
}

#[derive(Debug, Clone)]
pub struct Chapter {
    pub handle: u64,
}

impl Chapter {
    pub fn new(handle: u64) -> Self {
        Self { handle }
    }
}
