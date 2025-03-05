use std::io::Cursor;

use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
};
use kira::sound::static_sound::StaticSoundData;

#[derive(Asset, TypePath)]
pub struct SfxAsset {
    pub sound_data: StaticSoundData
}

pub struct SfxAssetLoader {
    _marker: std::marker::PhantomData<SfxAsset>,
}

impl Default for SfxAssetLoader {
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl AssetLoader for SfxAssetLoader {
    type Asset = SfxAsset;
    type Settings = ();
    type Error = Box<dyn std::error::Error + Send + Sync>;
    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let mut cursor = Cursor::new(bytes);
        let sfx_asset = SfxAsset {
            sound_data: StaticSoundData::from_cursor(cursor)?,
        };
        Ok(sfx_asset)
    }

    fn extensions(&self) -> &[&str] {
        &["wav", "ogg"]
    }
}