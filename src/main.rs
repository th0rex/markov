use std::{
    collections::HashMap,
    env,
    sync::{Arc, Mutex},
};

use markov::Chain;

use serenity::{
    model::{
        channel::Message,
        gateway::{Activity, Ready},
        id::{GuildId, MessageId},
    },
    prelude::*,
};

struct Handler {
    markov: Arc<Mutex<HashMap<GuildId, Chain<String>>>>,
}

impl EventHandler for Handler {
    fn message(&self, ctx: Context, msg: Message) {
        let channel = msg.channel_id;
        let guild = match msg.guild_id {
            Some(x) => x,
            None => return,
        };

        match msg.content.as_str() {
            "!markov" | "!blockchain" | "!ai" | "!quantum" => {
                let markov = self.markov.lock().unwrap();
                let markov = match markov.get(&guild) {
                    Some(x) => x,
                    None => {
                        msg.channel_id
                            .say(&ctx.http, "Error: markov chain doesnt exist")
                            .unwrap();
                        return;
                    }
                };

                let mut s = markov.generate_str();
                while s.trim().is_empty() {
                    s = markov.generate_str();
                }

                msg.channel_id.say(&ctx.http, s).unwrap();
            }
            "!exit" | "!save" => {
                if msg.author.id.0 != 157_149_752_327_143_425 {
                    msg.channel_id.say(&ctx.http, "No permissions :)").unwrap();
                    msg.react(ctx.http, "👎").unwrap();
                    return;
                }

                let markov = self.markov.lock().unwrap();
                for (k, v) in &*markov {
                    v.save(format!("./data/{}", k.0)).unwrap();
                }

                msg.react(ctx.http, "👍").unwrap();
            }
            x if !(msg.author.bot || x.starts_with('!') || x.starts_with(";;")) => {
                let mut markov = self.markov.lock().unwrap();
                let markov = markov.entry(guild).or_insert_with(|| Chain::of_order(2));
                markov.feed_str(x);
            }
            x if x.starts_with("!load") => {
                if msg.author.id.0 != 157_149_752_327_143_425 {
                    msg.channel_id.say(&ctx.http, "No permissions :)").unwrap();
                    msg.react(ctx.http, "👎").unwrap();
                    return;
                }

                let mut markov = self.markov.lock().unwrap();
                let markov = markov.entry(guild).or_insert_with(|| Chain::of_order(1));

                let query = msg.content.as_str();
                let query = &query[query.find(' ').unwrap_or(4) + 1..];
                let id = if !query.is_empty() {
                    MessageId(query.parse::<u64>().unwrap())
                } else {
                    msg.id
                };

                for msg in channel
                    .messages(ctx.http.clone(), |x| x.before(id).limit(100))
                    .unwrap()
                {
                    if !(msg.author.bot
                        || msg.content.starts_with('!')
                        || msg.content.starts_with(";;")
                        || msg.content.trim().is_empty())
                    {
                        markov.feed_str(&msg.content);
                    }
                }

                msg.react(ctx.http, "👍").unwrap();
            }
            _ => {}
        }
    }

    fn ready(&self, ctx: Context, _: Ready) {
        ctx.set_activity(Activity::playing("blockchain"));
    }
}

fn main() -> Result<(), std::io::Error> {
    let token = env::var("DISCORD_TOKEN").unwrap();

    let mut markov = HashMap::new();

    for x in std::fs::read_dir("./data")? {
        let entry = x?;
        if entry.file_type()?.is_file() {
            let x = entry
                .path()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .parse::<u64>()
                .unwrap();
            markov.insert(GuildId(x), Chain::load(entry.path())?);
        }
    }

    println!("Loaded {} guilds", markov.len());

    let mut client = Client::new(
        &token,
        Handler {
            markov: Arc::new(Mutex::new(markov)),
        },
    )
    .unwrap();

    client.start().unwrap();

    Ok(())
}
