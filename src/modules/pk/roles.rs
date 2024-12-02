use core::panic;
use std::collections::{HashMap, HashSet};

use pkrs::model::PkId;
use poise::serenity_prelude::{self as serenity, PartialGuild};
use tracing::debug;

use super::db::get_guild_settings_for_id;
use crate::types::{Context, Error};
use crate::util::{get_member_name, hex_to_color};

#[derive(Debug, Hash, Eq, PartialEq)]
struct MemberRole {
    id: Option<serenity::RoleId>,
    name: String,
    color: u32,
}

enum ChangeOperation {
    Create {
        name: String,
        color: u32,
    },
    Delete {
        id: serenity::RoleId,
        name: String,
    },
    Update {
        id: serenity::RoleId,
        name: String,
        color: u32,
    },
}

async fn get_desired_roles(
    system: &PkId,
    token: String,
) -> Result<HashMap<String, MemberRole>, Error> {
    let pk = pkrs::client::PkClient {
        token,
        ..Default::default()
    };

    let roles = pk
        .get_system_members(system)
        .await?
        .into_iter()
        .map(|m| MemberRole {
            id: None,
            name: format!(
                "{} (Alter)",
                get_member_name(&m)
                    .split(" (") // Remove parenthesised pronouns ' (she/her)' and such
                    .next() // get the first part of the split string
                    .unwrap()
            ),
            color: hex_to_color(m.color).0,
        })
        .map(|r| (r.name.to_owned(), r))
        .collect();

    Ok(roles)
}

fn get_current_roles(guild: PartialGuild) -> HashMap<String, MemberRole> {
    guild
        .roles
        .values()
        .filter(|v| v.name.ends_with(" (Alter)"))
        .map(|v| MemberRole {
            id: Some(v.id),
            name: v.name.clone(),
            color: v.colour.0,
        })
        .map(|v| (v.name.clone(), v))
        .collect()
}

fn get_ops(
    current: HashMap<String, MemberRole>,
    desired: HashMap<String, MemberRole>,
) -> Vec<ChangeOperation> {
    let all_roles: HashSet<&String> = HashSet::from_iter(current.keys().chain(desired.keys()));

    all_roles
        .into_iter()
        .filter_map(|role| {
            match (current.get(role), desired.get(role)) {
                // Update, only if color changed
                (Some(current), Some(desired)) => {
                    if current.color != desired.color {
                        Some(ChangeOperation::Update {
                            id: current.id.unwrap(),
                            name: current.name.clone(),
                            color: desired.color,
                        })
                    } else {
                        None
                    }
                }
                // Create
                (None, Some(desired)) => Some(ChangeOperation::Create {
                    name: desired.name.clone(),
                    color: desired.color,
                }),
                // Delete
                (Some(current), None) => Some(ChangeOperation::Delete {
                    id: current.id.unwrap(),
                    name: current.name.clone(),
                }),
                // Shit got fucked up aaaa
                (None, None) => panic!("current and desired are both None, shouldn't happen"),
            }
        })
        .collect()
}

#[poise::command(
    slash_command,
    guild_only = true,
    rename = "update-member-roles",
    default_member_permissions = "MANAGE_GUILD"
)]
pub(crate) async fn update_member_roles(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer_ephemeral().await?; // delay responding and make reply ephemeral

    let guild = ctx.partial_guild().await.unwrap();
    let gs = get_guild_settings_for_id(&ctx.data().db, guild.id.get())
        .await?
        .ok_or("PluralKit module not set-up, please run /setup-pk")?;

    let current_role_map = get_current_roles(guild.clone());
    let desired_role_map = get_desired_roles(
        &PkId(gs.system_id),
        gs.token.clone().unwrap_or("".to_owned()),
    )
    .await?;
    let ops = get_ops(current_role_map, desired_role_map);

    // TODO: actually handle errors
    // TODO: set mention permissions?
    for op in ops.iter() {
        match op {
            ChangeOperation::Update { id, name, color } => {
                guild
                    .edit_role(&ctx, id, serenity::EditRole::new().colour(*color))
                    .await?;

                debug!(
                    guild_id = guild.id.get(),
                    guild_name = guild.name,
                    "updated role: {}",
                    name,
                )
            }
            ChangeOperation::Create { name, color } => {
                guild
                    .create_role(&ctx, serenity::EditRole::new().name(name).colour(*color))
                    .await?;
                debug!(
                    guild_id = guild.id.get(),
                    guild_name = guild.name,
                    "created role: {}",
                    name
                )
            }
            ChangeOperation::Delete { id, name } => {
                guild.delete_role(&ctx, id).await?;
                debug!(
                    guild_id = guild.id.get(),
                    guild_name = guild.name,
                    "deleted_role: {}",
                    name
                )
            }
        };
    }

    // aggregate stats
    let (created, deleted, updated) =
        ops.iter()
            .fold((0, 0, 0), |(created, deleted, updated), op| match op {
                ChangeOperation::Create { .. } => (created + 1, deleted, updated),
                ChangeOperation::Delete { .. } => (created, deleted + 1, updated),
                ChangeOperation::Update { .. } => (created, deleted, updated + 1),
            });

    ctx.reply(format!(
        "roles updated, {} created, {} deleted, {} updated",
        created, deleted, updated
    ))
    .await?;
    Ok(())
}
