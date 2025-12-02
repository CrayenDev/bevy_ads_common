//! Mockup implementation of the AdManager trait.
//! Implements the AdManager trait for testing purposes.
use bevy_app::{App, PostStartup, Update};
use bevy_derive::Deref;
use bevy_ecs::{
    bundle::Bundle,
    children,
    component::Component,
    entity::Entity,
    hierarchy::ChildOf,
    lifecycle::Remove,
    observer::On,
    prelude::{ReflectComponent, ReflectResource},
    query::With,
    resource::Resource,
    schedule::{IntoScheduleConfigs, common_conditions::resource_exists},
    spawn::SpawnRelated,
    system::{Commands, In, Query, Res, ResMut, SystemParam},
};
use bevy_picking::events::{Click, Pointer};
use bevy_reflect::Reflect;
use bevy_time::{Time, Timer, TimerMode};
use bevy_ui::{
    AlignItems, BackgroundColor, FlexDirection, JustifyContent, JustifyItems, Node, PositionType,
    Val,
    widget::{Button, ImageNode, Text},
};
use std::time::Duration;

use crate::{AdManager, AdMessage, AdType};

#[derive(Debug, Resource, Reflect)]
#[reflect(Resource)]
pub struct MockupAds {
    pub initialized: bool,
    pub rewarded: AdDisplaySettings,
    pub interstitial: AdDisplaySettings,
    pub rewarded_ad_reward: Reward,
    pub loading_time_ms: u64,
}

#[derive(Debug, Reflect, Resource, Default)]
#[reflect(Resource)]
pub struct MockupFakeLoader {
    duration: Duration,
    rewarded: Option<Timer>,
    interstitial: Option<Timer>,
}

impl MockupFakeLoader {
    pub fn set_duration(&mut self, duration: Duration) {
        self.duration = duration;
        self.interstitial = None;
        self.rewarded = None;
    }
    pub fn is_loaded(&self, ad_type: AdType) -> bool {
        match ad_type {
            AdType::Rewarded => self
                .rewarded
                .as_ref()
                .is_some_and(|timer| timer.is_finished()),
            AdType::Interstitial => self
                .interstitial
                .as_ref()
                .is_some_and(|timer| timer.is_finished()),
            _ => true,
        }
    }
    pub fn start_load(&mut self, ad_type: AdType) {
        match ad_type {
            AdType::Rewarded => {
                self.rewarded = Some(Timer::new(self.duration, TimerMode::Once));
            }
            AdType::Interstitial => {
                self.interstitial = Some(Timer::new(self.duration, TimerMode::Once));
            }
            _ => {}
        }
    }
    fn reset(&mut self, ad_type: AdType) {
        match ad_type {
            AdType::Rewarded => {
                self.rewarded = None;
            }
            AdType::Interstitial => {
                self.interstitial = None;
            }
            _ => {}
        }
    }
    fn update(mut loader: ResMut<MockupFakeLoader>, time: Res<Time>) {
        if let Some(ref mut timer) = loader.rewarded {
            timer.tick(time.delta());
            if timer.just_finished() {
                crate::write_event_to_queue(AdMessage::AdLoaded {
                    ad_type: AdType::Rewarded,
                });
            }
        }
        if let Some(ref mut timer) = loader.interstitial {
            timer.tick(time.delta());
            if timer.just_finished() {
                crate::write_event_to_queue(AdMessage::AdLoaded {
                    ad_type: AdType::Interstitial,
                });
            }
        }
    }
}

#[derive(Debug, Reflect, Clone)]
pub struct AdDisplaySettings {
    pub display: AdDisplay,
    pub show_time_left: bool,
    pub auto_close: bool,
    pub duration_ms: u64,
}

impl Default for AdDisplaySettings {
    fn default() -> Self {
        Self {
            display: AdDisplay::SolidBackgroundWithText(
                BackgroundColor(bevy_color::palettes::tailwind::ZINC_500.into()),
                "Displaying an ad".to_string(),
            ),
            show_time_left: true,
            auto_close: false,
            duration_ms: 3500,
        }
    }
}

/// Reward for displaying an rewarded ad.
#[derive(Debug, Reflect, Clone)]
pub struct Reward {
    pub amount: i32,
    pub type_name: String,
}

impl Default for Reward {
    fn default() -> Self {
        Self {
            amount: 1,
            type_name: "default".to_string(),
        }
    }
}

/// Settings for displaying an fullscreen ad.
#[derive(Debug, Reflect, Clone)]
pub enum AdDisplay {
    /// Display a fullscreen ad with a solid background color.
    SolidBackground(BackgroundColor),
    /// Display a fullscreen ad with a solid background color and a text message.
    SolidBackgroundWithText(BackgroundColor, String),
    /// Display a fullscreen ad with an image.
    Image(bevy_asset::Handle<bevy_image::Image>),
}

impl Default for MockupAds {
    fn default() -> Self {
        Self {
            initialized: false,
            interstitial: AdDisplaySettings::default(),
            rewarded: AdDisplaySettings::default(),
            rewarded_ad_reward: Reward::default(),
            loading_time_ms: 1000,
        }
    }
}

pub(crate) fn plugin(app: &mut App) {
    app.register_type::<MockupAds>()
        .init_resource::<MockupAds>()
        .register_type::<MockupFakeLoader>()
        .init_resource::<MockupFakeLoader>()
        .register_type::<MockupAdComponent>()
        .register_type::<MockupAdType>()
        .add_systems(Update, show_ads)
        .add_systems(
            Update,
            MockupFakeLoader::update.run_if(resource_exists::<MockupFakeLoader>),
        )
        .add_systems(PostStartup, init)
        .add_observer(on_despawn)
        .add_observer(close_clicked);
}

fn init(mut ads: MockupAdsSystem) {
    ads.initialize();
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct MockupAdComponent {
    pub timer: bevy_time::Timer,
    pub auto_close: bool,
}

#[derive(Component, Reflect, Deref)]
#[reflect(Component)]
pub struct MockupAdType(AdType);

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct MockupAdTimeLeftText;

#[derive(SystemParam)]
pub struct MockupAdsSystem<'w, 's> {
    pub r: ResMut<'w, MockupAds>,
    pub cmd: Commands<'w, 's>,
    pub timer: ResMut<'w, MockupFakeLoader>,
}

impl MockupAdsSystem<'_, '_> {
    pub fn show_fullscreen_ad(&mut self, ad_type: AdType) -> bool {
        if !self.is_initialized() {
            return false;
        }
        if !self.timer.is_loaded(ad_type) {
            return false;
        }
        let settings = match ad_type {
            AdType::Banner => return false,
            AdType::Interstitial => &self.r.interstitial,
            AdType::Rewarded => &self.r.rewarded,
        };
        let show_time_left = settings.show_time_left;
        let auto_close = settings.auto_close;
        let duration = settings.duration_ms;
        let mut ss = match &settings.display {
            AdDisplay::SolidBackground(background_color) => self
                .cmd
                .spawn((ad_bundle(duration, ad_type, auto_close), *background_color)),
            AdDisplay::SolidBackgroundWithText(background_color, text) => self.cmd.spawn((
                ad_bundle(duration, ad_type, auto_close),
                *background_color,
                children![Text::new(text)],
            )),
            AdDisplay::Image(handle) => self.cmd.spawn((
                ad_bundle(duration, ad_type, auto_close),
                ImageNode::new(handle.clone()),
            )),
        };
        if show_time_left {
            ss.with_child(time_left());
        }
        true
    }
}

impl AdManager for MockupAdsSystem<'_, '_> {
    fn is_initialized(&self) -> bool {
        self.r.initialized
    }

    fn initialize(&mut self) -> bool {
        if self.r.initialized {
            return true;
        }
        self.timer
            .set_duration(Duration::from_millis(self.r.loading_time_ms));

        self.r.initialized = true;
        crate::write_event_to_queue(AdMessage::Initialized { success: true });
        true
    }

    fn show_banner(&mut self) -> bool {
        self.cmd.spawn(banner_bundle());
        true
    }

    fn show_interstitial(&mut self) -> bool {
        self.show_fullscreen_ad(AdType::Interstitial)
    }

    fn show_rewarded(&mut self) -> bool {
        self.show_fullscreen_ad(AdType::Rewarded)
    }

    fn hide_banner(&mut self) -> bool {
        self.cmd.run_system_cached_with(hide_ad, AdType::Banner);
        true
    }

    fn hide_interstitial(&mut self) -> bool {
        self.cmd
            .run_system_cached_with(hide_ad, AdType::Interstitial);
        true
    }

    fn hide_rewarded(&mut self) -> bool {
        self.cmd.run_system_cached_with(hide_ad, AdType::Rewarded);
        true
    }

    fn load_banner(&mut self, _ad_id: &str) -> bool {
        true
    }

    fn load_interstitial(&mut self, _ad_id: &str) -> bool {
        self.timer.start_load(AdType::Interstitial);
        true
    }

    fn load_rewarded(&mut self, _ad_id: &str) -> bool {
        self.timer.start_load(AdType::Rewarded);
        true
    }

    fn is_interstitial_ready(&self) -> bool {
        if !self.is_initialized() {
            return false;
        }
        self.timer.is_loaded(AdType::Interstitial)
    }

    fn is_rewarded_ready(&self) -> bool {
        if !self.is_initialized() {
            return false;
        }
        self.timer.is_loaded(AdType::Rewarded)
    }
}

fn show_ads(
    mut q: Query<(Entity, &mut MockupAdComponent, &MockupAdType)>,
    mut qq: Query<&mut Text, With<MockupAdTimeLeftText>>,
    time: Res<Time>,
    mut commands: Commands,
    cfg: Res<MockupAds>,
) {
    for (entity, mut component, ad_type) in q.iter_mut() {
        component.timer.tick(time.delta());
        if component.timer.just_finished() {
            if ad_type.eq(&AdType::Rewarded) {
                crate::write_event_to_queue(AdMessage::RewardedAdEarnedReward {
                    amount: cfg.rewarded_ad_reward.amount,
                    reward_type: cfg.rewarded_ad_reward.type_name.clone(),
                });
            }
            if component.auto_close {
                commands.entity(entity).try_despawn();
            } else {
                commands.spawn((close_btn(), ChildOf(entity)));
            }
        } else {
            for mut text in qq.iter_mut() {
                text.0 = format!("{:.2}s left", component.timer.remaining_secs());
            }
        }
    }
}

fn hide_ad(In(ad_type): In<AdType>, mut commands: Commands, q: Query<(Entity, &MockupAdType)>) {
    for (entity, component_ad_type) in q.iter() {
        if !component_ad_type.eq(&ad_type) {
            continue;
        }
        let Ok(mut e) = commands.get_entity(entity) else {
            continue;
        };
        e.try_despawn();
    }
}

fn on_despawn(
    t: On<Remove, MockupAdType>,
    q: Query<&MockupAdType>,
    timer: Option<ResMut<MockupFakeLoader>>,
) {
    let Ok(ad_type_component) = q.get(t.entity) else {
        bevy_log::warn!("Failed to get component info");
        return;
    };
    let ad_type = **ad_type_component;
    crate::write_event_to_queue(AdMessage::AdClosed { ad_type });
    if let Some(mut timer) = timer {
        timer.reset(ad_type_component.0);
    }
}

fn ad_bundle(duration_ms: u64, ad_type: AdType, auto_close: bool) -> impl Bundle {
    (
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            justify_items: JustifyItems::Stretch,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(10.0),
            position_type: PositionType::Absolute,
            ..Default::default()
        },
        MockupAdComponent {
            timer: bevy_time::Timer::new(Duration::from_millis(duration_ms), TimerMode::Once),
            auto_close,
        },
        MockupAdType(ad_type),
        bevy_ui::ZIndex(500),
    )
}

fn time_left() -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(5.0),
            left: Val::Px(5.0),
            ..Default::default()
        },
        Text::new(""),
        MockupAdTimeLeftText,
        bevy_ui::widget::TextShadow::default(),
    )
}

fn close_btn() -> impl Bundle {
    (
        Button,
        Node {
            width: Val::Px(30.0),
            height: Val::Px(30.0),
            position_type: PositionType::Absolute,
            top: Val::Px(5.0),
            right: Val::Px(5.0),
            ..Default::default()
        },
        BackgroundColor(bevy_color::palettes::tailwind::RED_400.into()),
    )
}

fn close_clicked(
    t: On<Pointer<Click>>,
    q: Query<&ChildOf, With<Button>>,
    p_q: Query<&MockupAdType>,
    mut ads: MockupAdsSystem,
) {
    let Ok(p) = q.get(t.entity) else {
        return;
    };
    let Ok(ad) = p_q.get(p.0) else {
        return;
    };
    ads.hide_ad(ad.0);
}

fn banner_bundle() -> impl Bundle {
    (
        Node {
            width: Val::Px(100.0),
            height: Val::Px(30.0),
            bottom: Val::Px(0.0),
            justify_content: JustifyContent::Center,
            justify_items: JustifyItems::Stretch,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(10.0),
            position_type: PositionType::Absolute,

            ..Default::default()
        },
        MockupAdType(AdType::Banner),
        bevy_ui::ZIndex(500),
    )
}
