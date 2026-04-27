use crate::{MakeMkv, command::ItemAttribute, error::MakeMkvError};
use log::debug;
use std::time::Duration;

pub trait Rippable {
    /// Returns the handle of the object
    fn handle(&self) -> u64;

    /// Checks if the item will be ripped
    fn set_enabled(&mut self, enabled: bool);
}

#[derive(Debug, Clone)]
pub struct TitleList {
    _handle: u64,
    size: u32,
    pub titles: Vec<Option<Title>>,
}

impl TitleList {
    pub(crate) fn new(handle: u64, size: u32) -> Self {
        Self {
            _handle: handle,
            titles: vec![None; size as usize],
            size,
        }
    }

    pub(crate) fn add_title(
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

    pub(crate) fn add_track(&mut self, title_index: u32, track_index: u32, handle: u64) {
        if title_index >= self.size {
            return;
        }

        if let Some(title) = &mut self.titles[title_index as usize] {
            title.add_track(track_index, handle);
        }
    }

    pub(crate) fn add_chapter(&mut self, title_index: u32, chapter_index: u32, handle: u64) {
        if title_index >= self.size {
            return;
        }

        if let Some(title) = &mut self.titles[title_index as usize] {
            title.chapters.add_chapter(chapter_index, handle);
        }
    }

    /// Get all the data related to the titles, including name and length
    pub(crate) async fn get_data(&mut self, makemkv: &mut MakeMkv) -> Result<(), MakeMkvError> {
        for title in self.titles.iter_mut().flatten() {
            title.get_data(makemkv).await?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Title {
    pub chapters: ChapterList,
    pub tracks: Vec<Option<Track>>,
    track_size: u32,
    handle: u64,
    pub name: Option<String>,
    pub duration: Option<Duration>,
    pub disc_size: Option<String>,
    pub enabled: bool,
}

impl Title {
    fn new(handle: u64, chapter_handle: u64, chapter_size: u32, track_size: u32) -> Self {
        Self {
            chapters: ChapterList::new(chapter_handle, chapter_size),
            tracks: vec![None; track_size as usize],
            handle,
            track_size,
            name: None,
            duration: None,
            disc_size: None,
            enabled: false,
        }
    }

    fn add_track(&mut self, index: u32, handle: u64) {
        if index < self.track_size {
            self.tracks[index as usize] = Some(Track::new(handle));
        }
    }

    async fn get_data(&mut self, makemkv: &mut MakeMkv) -> Result<(), MakeMkvError> {
        self.name = makemkv
            .get_item_info(self.handle, ItemAttribute::Name)
            .await?;
        debug!("Title name: {:?}", &self.name);

        self.duration = parse_duration(
            makemkv
                .get_item_info(self.handle, ItemAttribute::Duration)
                .await?,
        );
        debug!("Title duration: {:?}", &self.duration);

        // Get state of title
        self.enabled = makemkv.is_enabled(self).await?;

        self.disc_size = makemkv
            .get_item_info(self.handle, ItemAttribute::DiskSize)
            .await?;

        self.chapters.get_data(makemkv).await?;
        for track in self.tracks.iter_mut().flatten() {
            track.get_data(makemkv).await?;
        }

        Ok(())
    }
}

impl Rippable for Title {
    fn handle(&self) -> u64 {
        self.handle
    }

    fn set_enabled(&mut self, enable: bool) {
        self.enabled = enable;
    }
}

#[derive(Debug, Clone)]
pub struct Track {
    handle: u64,
    track_type: Option<String>,
    codec: Option<String>,
    pub enabled: bool,
}

impl Track {
    fn new(handle: u64) -> Self {
        Self {
            handle,
            track_type: None,
            codec: None,
            enabled: false,
        }
    }

    async fn get_data(&mut self, makemkv: &mut MakeMkv) -> Result<(), MakeMkvError> {
        self.track_type = makemkv
            .get_item_info(self.handle, ItemAttribute::Type)
            .await?;

        self.codec = makemkv
            .get_item_info(self.handle, ItemAttribute::CodecLong)
            .await?;

        // Get state of track
        self.enabled = makemkv.is_enabled(self).await?;

        Ok(())
    }
}

impl Rippable for Track {
    fn handle(&self) -> u64 {
        self.handle
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

#[derive(Debug, Clone, Default)]
pub struct ChapterList {
    _handle: u64,
    size: u32,
    chapters: Vec<Option<Chapter>>,
}

impl ChapterList {
    fn new(handle: u64, size: u32) -> Self {
        Self {
            _handle: handle,
            chapters: vec![None; size as usize],
            size,
        }
    }

    fn add_chapter(&mut self, index: u32, handle: u64) {
        if index >= self.size {
            return;
        }

        self.chapters[index as usize] = Some(Chapter::new(handle));
    }

    async fn get_data(&mut self, makemkv: &mut MakeMkv) -> Result<(), MakeMkvError> {
        for chapter in self.chapters.iter_mut().flatten() {
            chapter.get_data(makemkv).await?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct Chapter {
    pub handle: u64,
    pub name: Option<String>,
    pub datetime: Option<Duration>,
}

impl Chapter {
    fn new(handle: u64) -> Self {
        Self {
            handle,
            ..Default::default()
        }
    }

    async fn get_data(&mut self, makemkv: &mut MakeMkv) -> Result<(), MakeMkvError> {
        self.name = makemkv
            .get_item_info(self.handle, ItemAttribute::Name)
            .await?;

        self.datetime = parse_duration(
            makemkv
                .get_item_info(self.handle, ItemAttribute::DateTime)
                .await?,
        );

        Ok(())
    }
}

fn parse_duration(s: Option<String>) -> Option<Duration> {
    let duration_data = s?
        .split(':')
        .map(|s| s.parse::<u64>().ok())
        .collect::<Option<Vec<u64>>>()?;

    if duration_data.len() == 3 {
        Some(
            Duration::from_hours(duration_data[0])
                .saturating_add(Duration::from_mins(duration_data[1]))
                .saturating_add(Duration::from_secs(duration_data[2])),
        )
    } else {
        None
    }
}
