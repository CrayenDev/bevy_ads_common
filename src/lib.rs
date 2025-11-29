#![doc = include_str!("../README.md")]
use std::fmt::Display;

use bevy_app::{App, FixedUpdate, Plugin};
use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;
use crossbeam::queue::SegQueue;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[cfg(feature = "mockup")]
mod mockup;

pub mod prelude {
    #[cfg(feature = "mockup")]
    pub use crate::mockup::{
        AdDisplay, AdDisplaySettings, MockupAdComponent, MockupAdText, MockupAdType, MockupAds,
        MockupAdsSystem,
    };
    pub use crate::{AdManager, AdMessage, AdType, AdsCommonPlugin};
}

static EVENT_QUEUE: Lazy<SegQueue<AdMessage>> = Lazy::new(SegQueue::new);

/// Write an event to the queue.
/// In almost all cases this should be called only by the ads implementation plugin.
pub fn write_event_to_queue(event: AdMessage) {
    EVENT_QUEUE.push(event);
}

/// Events that can be triggered by Ad system operations
#[derive(Message, Debug, Clone, Reflect, Serialize, Deserialize)]
pub enum AdMessage {
    /// Ad system completed initialization.
    Initialized { success: bool },
    /// Consent was gathered.
    ConsentGathered { success: bool, error: String },
    /// Ad was loaded.
    AdLoaded { ad_type: String },
    /// Ad failed to load.
    AdFailedToLoad { ad_type: String, error: String },
    /// Ad was opened.
    AdOpened { ad_type: String },
    /// Ad was closed.
    AdClosed { ad_type: String },
    /// Rewarded ad earned reward.
    RewardedAdEarnedReward { amount: i32, reward_type: String },
}

/// Ad type description enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize)]
pub enum AdType {
    /// Banner ad type
    Banner,
    /// Interstitial ad type
    Interstitial,
    /// Rewarded ad type
    Rewarded,
}

impl Display for AdType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdType::Banner => write!(f, "banner"),
            AdType::Interstitial => write!(f, "interstitial"),
            AdType::Rewarded => write!(f, "rewarded"),
        }
    }
}

/// Trait for managing ads system.
pub trait AdManager {
    /// Initialize the AdManager.
    /// Returns true if it was able to start initialization process.
    fn initialize(&mut self) -> bool;
    /// Check if the AdManager is initialized.
    fn is_initialized(&self) -> bool;
    /// Load an ad of the specified type and ID.
    /// Returns true if the ad loading process was successfully started.
    fn load_ad(&mut self, ad_type: AdType, ad_id: &str) -> bool {
        match ad_type {
            AdType::Banner => self.load_banner(ad_id),
            AdType::Interstitial => self.load_interstitial(ad_id),
            AdType::Rewarded => self.load_rewarded(ad_id),
        }
    }
    /// Show an ad of the specified type.
    /// Returns true if the ad was successfully shown.
    fn show_ad(&mut self, ad_type: AdType) -> bool {
        if !self.is_ad_ready(ad_type) {
            return false;
        }
        match ad_type {
            AdType::Banner => self.show_banner(),
            AdType::Interstitial => self.show_interstitial(),
            AdType::Rewarded => self.show_rewarded(),
        }
    }
    /// Hide an ad of the specified type.
    /// Returns true if the ad was successfully hidden.
    fn hide_ad(&mut self, ad_type: AdType) -> bool {
        match ad_type {
            AdType::Banner => self.hide_banner(),
            AdType::Interstitial => self.hide_interstitial(),
            AdType::Rewarded => self.hide_rewarded(),
        }
    }
    /// Check if an ad of the specified type is ready to be shown.
    /// Returns true if the ad is ready.
    fn is_ad_ready(&self, ad_type: AdType) -> bool {
        match ad_type {
            AdType::Banner => self.is_banner_ready(),
            AdType::Interstitial => self.is_interstitial_ready(),
            AdType::Rewarded => self.is_rewarded_ready(),
        }
    }
    /// Show a banner ad.
    /// Returns true if the ad was successfully shown.
    fn show_banner(&mut self) -> bool;
    /// Show an interstitial ad.
    /// Returns true if the ad was successfully shown.
    fn show_interstitial(&mut self) -> bool;
    /// Show a rewarded ad.
    /// Returns true if the ad was successfully shown.
    fn show_rewarded(&mut self) -> bool;
    /// Hide a banner ad.
    /// Returns true if the ad was successfully hidden.
    fn hide_banner(&mut self) -> bool;
    /// Hide an interstitial ad.
    /// Returns true if the ad was successfully hidden.
    fn hide_interstitial(&mut self) -> bool;
    /// Hide a rewarded ad.
    /// Returns true if the ad was successfully hidden.
    fn hide_rewarded(&mut self) -> bool;
    /// Load a banner ad.
    /// Returns true if the ad was successfully loaded.
    fn load_banner(&mut self, ad_id: &str) -> bool;
    /// Load an interstitial ad.
    /// Returns true if the ad was successfully loaded.
    fn load_interstitial(&mut self, ad_id: &str) -> bool;
    /// Load a rewarded ad.
    /// Returns true if the ad was successfully loaded.
    fn load_rewarded(&mut self, ad_id: &str) -> bool;
    /// Is a banner ad ready to be shown?
    fn is_banner_ready(&self) -> bool {
        true
    }
    /// Is an interstitial ad ready to be shown?
    fn is_interstitial_ready(&self) -> bool {
        false
    }
    /// Is a rewarded ad ready to be shown?
    fn is_rewarded_ready(&self) -> bool {
        false
    }

    /// Get the width of the banner ad.
    fn get_banner_width(&self, _ad_id: &str) -> i32 {
        100
    }

    /// Get the height of the banner ad.
    fn get_banner_height(&self, _ad_id: &str) -> i32 {
        50
    }
}

/// Basic plugin for managing ads.
/// It provides a set of methods alongside a optional mockup ads implementation.
pub struct AdsCommonPlugin;

impl Plugin for AdsCommonPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<AdMessage>()
            .add_systems(FixedUpdate, handle_events)
            .register_type::<AdMessage>();
        #[cfg(feature = "mockup")]
        app.add_plugins(mockup::plugin);
    }
}

fn handle_events(mut writer: MessageWriter<AdMessage>) {
    while let Some(ev) = EVENT_QUEUE.pop() {
        writer.write(ev);
    }
}
