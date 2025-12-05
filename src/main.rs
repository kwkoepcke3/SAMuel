mod fetch;
mod steam;

use clap::{Args, Parser, Subcommand};
use dotenv::dotenv;
use std::{env, fs, path::Path};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    #[command(subcommand)]
    command: CliCommandsSub,
}

#[derive(Subcommand, Debug)]
#[clap(rename_all = "snake_case")]
enum CliCommandsSub {
    Games(GamesArgs),
    Achievements(AchievementsArgs),
    UpdateCache,
}

#[derive(Args, Debug)]
struct GamesArgs {
    #[command(subcommand)]
    command: GamesArgsSub,
}

#[derive(Subcommand, Debug)]
enum GamesArgsSub {
    Find(GamesFindArgs),
    List(GamesListArgs),
}

#[derive(Args, Debug)]
struct GamesFindArgs {
    #[arg(long, help = "Switch to find specific game by name instead of by id")]
    by_name: bool,

    game_id: String,
}

#[derive(Args, Debug)]
struct GamesListArgs {
    #[arg(long)]
    no_header: bool,

    #[arg(long)]
    sort_by: Option<usize>,
}
#[derive(Args, Debug)]
struct AchievementsArgs {
    #[command(subcommand)]
    command: AchievementsArgsSub,
}

#[derive(Args, Debug)]
struct AchievementsResetAllArgs {
    game_id: String,
}

#[derive(Subcommand, Debug)]
enum AchievementsArgsSub {
    List(AchievementsListArgs),
    Trigger(AchievementsTriggerArgs),
    Clear(AchievementsClearArgs),
    ResetAll(AchievementsResetAllArgs),
}

#[derive(Args, Debug)]
struct AchievementsTriggerArgs {
    game_id: String,

    achievement_id: String,
}

#[derive(Args, Debug)]
struct AchievementsClearArgs {
    game_id: String,

    achievement_id: String,
}

#[derive(Args, Debug)]
struct AchievementsListArgs {
    game_id: String,

    #[arg(long, help = "instead of game_id being an id, provide a game name")]
    by_game_name: bool,

    #[arg(long, help = "list only the given achievement by achievement_id")]
    achievement_id: Option<String>,

    #[arg(long, help = "list the full descriptions instead of truncating")]
    full: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let api_key = env::var("API_KEY").unwrap_or_else(|_| panic!("ENV VARIABLE API_KEY REQUIRED!"));
    let steam_id =
        env::var("STEAM_ID").unwrap_or_else(|_| panic!("ENV VARIABLE STEAM_ID REQUIRED!"));
    let data_dir_s = env::var("DATA_DIR")
        .unwrap_or_else(|_| (env::var("HOME").unwrap() + "/.local/share/samuel").to_owned());

    let data_dir = Path::new(&data_dir_s);
    let cache_path = data_dir.join("samuel.cache");

    fs::create_dir_all(&data_dir).expect("Could not create data dir!");

    let args = CliArgs::parse();

    match args.command {
        CliCommandsSub::Games(games_args) => match games_args.command {
            GamesArgsSub::Find(games_find_args) => {
                let owned_games = fetch::get_owned_games(&api_key, &steam_id, &cache_path).await?;
                let found = owned_games.response.games.iter().find(|game| {
                    if games_find_args.by_name {
                        game.name == games_find_args.game_id
                    } else {
                        game.appid.to_string() == games_find_args.game_id
                    }
                });

                match found {
                    Some(found) => println!("{found}"),
                    None => {}
                }
            }
            GamesArgsSub::List(games_list_args) => {
                let mut owned_games =
                    fetch::get_owned_games(&api_key, &steam_id, &cache_path).await?;

                if !games_list_args.no_header {
                    fetch::print_header();
                }

                if games_list_args.sort_by.is_some() {
                    owned_games.response.games.sort_by(|game1, game2| {
                        match games_list_args.sort_by {
                            Some(0) => game1.appid.cmp(&game2.appid),
                            Some(1) => game1.name.cmp(&game2.name),
                            Some(2) => game1.playtime_forever.cmp(&game2.playtime_forever),
                            _ => panic!("sort index out of range!"),
                        }
                    });
                }

                for game in owned_games.response.games {
                    println!("{game}");
                }
            }
            _ => todo!("implement some errors"),
        },
        CliCommandsSub::Achievements(achievements_args) => match achievements_args.command {
            AchievementsArgsSub::List(achievements_list_args) => {
                let owned_games = fetch::get_owned_games(&api_key, &steam_id, &cache_path).await?;
                let found = owned_games.response.games.iter().find(|game| {
                    if achievements_list_args.by_game_name {
                        game.name == achievements_list_args.game_id
                    } else {
                        game.appid.to_string() == achievements_list_args.game_id
                    }
                });

                if found.is_none() {
                    println!("{} not found", achievements_list_args.game_id);
                    return Ok(()); //TODO error
                }

                let achievements = steam::get_achievements(&(found.unwrap().appid as u32)).await;

                steam::print_get_achievements_header();
                match achievements_list_args.achievement_id {
                    Some(id) => {
                        let found = achievements
                            .iter()
                            .find(|achievement| achievement.achievement_id == id)
                            .unwrap(); //TODO error
                        steam::print_achievement_full(found);
                    }
                    None => {
                        achievements.iter().for_each(|achievement| {
                            if achievements_list_args.full {
                                steam::print_achievement_full(achievement)
                            } else {
                                println!("{achievement}")
                            }
                        });
                    }
                }
            }
            AchievementsArgsSub::Clear(achievements_clear_args) => steam::clear_achievement(
                &achievements_clear_args.game_id.parse().unwrap(), //TODO error
                achievements_clear_args.achievement_id,
            ),
            AchievementsArgsSub::Trigger(achievements_trigger_args) => steam::trigger_achievement(
                &achievements_trigger_args.game_id.parse().unwrap(),
                achievements_trigger_args.achievement_id,
            ),
            AchievementsArgsSub::ResetAll(achievements_clear_all_args) => {
                let AchievementsResetAllArgs { game_id } = achievements_clear_all_args;
                let appid = &game_id.parse().unwrap();
                let achievements = steam::get_achievements(&appid).await;
                for achievement in achievements {
                    steam::clear_achievement(&appid, achievement.achievement_id)
                }
            }
        },
        CliCommandsSub::UpdateCache => {
            fetch::get_owned_games_direct(&api_key, &steam_id, &cache_path).await?;
            println!("Updated cache")
        }
    }

    Ok(())
}
