//! Default `VideoProvider` methods for adapters that do not override them.
use rullst_media::*;
use std::sync::Mutex;

struct Minimal {
    fail_update: bool,
    titles: Mutex<Vec<String>>,
}

fn video(title: &str) -> RemoteVideo {
    RemoteVideo {
        id: VideoId::new("12345678-1234-4234-8234-123456789abc").unwrap(),
        library: LibraryId::new(7).unwrap(),
        title: title.to_owned(),
        description: String::new(),
        processing: Processing::AwaitingUpload,
        length_seconds: 0,
        mp4_720p: false,
    }
}

impl VideoProvider for Minimal {
    fn binding(&self) -> ProviderBinding {
        ProviderBinding {
            library: LibraryId::new(7).unwrap(),
            mode: ProviderMode::Offline,
            environment: Reference::new("defaults").unwrap(),
        }
    }
    async fn create(&self, marker: &str) -> Result<RemoteVideo, MediaError> {
        Ok(video(marker))
    }
    async fn find_created(&self, _: &str) -> Result<Option<RemoteVideo>, MediaError> {
        Ok(None)
    }
    async fn get(&self, _: &VideoId) -> Result<Option<RemoteVideo>, MediaError> {
        Ok(None)
    }
    async fn update(&self, _: &VideoId, metadata: &Metadata) -> Result<(), MediaError> {
        if self.fail_update {
            return Err(MediaError::Unavailable);
        }
        self.titles
            .lock()
            .unwrap()
            .push(metadata.title().to_owned());
        Ok(())
    }
    async fn delete(&self, _: &VideoId) -> Result<(), MediaError> {
        Ok(())
    }
    fn upload(&self, _: &VideoId, _: i64, _: u32) -> Result<UploadGrant, MediaError> {
        Err(MediaError::Unsupported)
    }
    fn playback(
        &self,
        _: &VideoId,
        _: i64,
        _: u32,
        _: PlaybackKind,
    ) -> Result<PlaybackGrant, MediaError> {
        Err(MediaError::Unsupported)
    }
}

#[tokio::test]
async fn default_listing_is_unsupported_and_titled_creation_composes() {
    let provider = Minimal {
        fail_update: false,
        titles: Mutex::new(Vec::new()),
    };
    assert!(matches!(
        provider.list(&ListQuery::new(1, 10).unwrap()).await,
        Err(MediaError::Unsupported)
    ));
    let metadata = Metadata::new("Lesson one", "Notes").unwrap();
    let created = provider
        .create_with_metadata("rullst-video-marker", &metadata)
        .await
        .unwrap();
    assert_eq!(
        (created.title.as_str(), created.description.as_str()),
        ("Lesson one", "Notes")
    );
    assert_eq!(*provider.titles.lock().unwrap(), ["Lesson one"]);

    let failing = Minimal {
        fail_update: true,
        titles: Mutex::new(Vec::new()),
    };
    let created = failing
        .create_with_metadata("rullst-video-marker", &metadata)
        .await
        .unwrap();
    assert_eq!(created.title, "rullst-video-marker");
    assert!(created.description.is_empty());
}
