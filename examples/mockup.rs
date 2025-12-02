//! Simple example that loads the tilemap and once is loaded it creates a sprite with it.

use bevy::{color, prelude::*};
use bevy_ads_common::prelude::*;

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
#[require(Text)]
struct AdMessagesHolder(Vec<String>);

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
#[require(Text)]
struct AdButtonText;

fn main() {
    App::new()
        .register_type::<AdMessagesHolder>()
        .register_type::<AdButtonText>()
        .add_plugins((DefaultPlugins, AdsCommonPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, on_message)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(50.0)),
                justify_content: JustifyContent::SpaceAround,
                justify_items: JustifyItems::Stretch,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                position_type: PositionType::Absolute,
                ..Default::default()
            },
            children![
                (
                    Node {
                        width: Val::Px(100.0),
                        height: Val::Px(50.0),
                        justify_content: JustifyContent::Center,
                        justify_items: JustifyItems::Stretch,

                        align_items: AlignItems::Center,
                        ..Default::default()
                    },
                    BorderRadius::all(Val::Px(10.0)),
                    BackgroundColor(color::palettes::tailwind::AMBER_400.into()),
                    Button,
                    children![(Text::new("Load Ad"), AdButtonText)]
                ),
                (AdMessagesHolder::default())
            ],
        ))
        .observe(on_click);
}

fn on_click(
    _t: On<Pointer<Press>>,
    mut ads: MockupAdsSystem,
    mut q: Query<&mut Visibility, With<Button>>,
) {
    if ads.is_interstitial_ready() {
        ads.show_interstitial();
    } else {
        for mut v in q.iter_mut() {
            v.set_if_neq(Visibility::Hidden);
        }
        ads.load_interstitial("");
    }
}

fn on_message(
    mut messages: MessageReader<AdMessage>,
    mut q: Query<(&mut AdMessagesHolder, &mut Text)>,
    mut btn_texts: Query<&mut Text, (With<AdButtonText>, Without<AdMessagesHolder>)>,
    mut q2: Query<&mut Visibility, With<Button>>,
    time: Res<Time>,
) {
    for message in messages.read() {
        for (mut ad_messages, mut text) in q.iter_mut() {
            ad_messages
                .0
                .push(format!("{:.2}s: {:?}", time.elapsed_secs(), message));
            text.0.clear();
            text.0.push_str("Ads events:\n");
            for mes in ad_messages.0.iter().rev() {
                text.0.push_str(mes);
                text.0.push('\n');
            }
        }
        if let AdMessage::AdLoaded { ad_type: _ } = message {
            for mut v in q2.iter_mut() {
                v.set_if_neq(Visibility::Inherited);
            }
            for mut text in btn_texts.iter_mut() {
                text.0.clear();
                text.0.push_str("Show Ad");
            }
        }
        if let AdMessage::AdClosed { ad_type: _ } = message {
            for mut text in btn_texts.iter_mut() {
                text.0.clear();
                text.0.push_str("Load Ad");
            }
        }
    }
}
