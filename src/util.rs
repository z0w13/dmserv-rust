use pkrs::model::Member;
use poise::serenity_prelude::{self as serenity};

pub(crate) fn hex_to_color(hex: Option<String>) -> serenity::Colour {
    return match hex {
        Some(hex) => serenity::Colour(
            u32::from_str_radix(hex.trim_start_matches("#"), 16)
                .unwrap_or(serenity::colours::roles::DEFAULT.0),
        ),
        None => serenity::colours::roles::DEFAULT,
    };
}

pub(crate) fn get_member_name(member: &Member) -> String {
    return member
        .display_name
        .to_owned()
        .unwrap_or(member.name.to_owned());
}
