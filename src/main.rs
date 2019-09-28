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

const ORDER: usize = 2;

struct Handler {
    markov: Arc<Mutex<HashMap<GuildId, Chain<String>>>>,
}

macro_rules! gen {
    ($m:expr, $s:expr, $x:expr) => {
        if let Some(i) = $s {
            $m.generate_str_from_token(&$x[i + 1..])
        } else {
            $m.generate_str()
        }
    };
}

impl EventHandler for Handler {
    fn message(&self, ctx: Context, msg: Message) {
        let channel = msg.channel_id;
        let guild = match msg.guild_id {
            Some(x) => x,
            None => return,
        };

        match msg.content.as_str() {
            x if x.starts_with("!markov") || x.starts_with("!ai") => {
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

                let maybe_start = x.find(' ');

                let mut s = gen!(markov, maybe_start, x);
                let mut i = 0;
                while i < 100 && (s.trim().is_empty() || s.bytes().len() > 2000) {
                    s = gen!(markov, maybe_start, x);
                    i += 1;
                }

                if i == 100 {
                    msg.channel_id
                        .say(&ctx.http, "Couldn't generate phrase??")
                        .unwrap();
                } else {
                    msg.channel_id.say(&ctx.http, s).unwrap();
                }
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
                let markov = markov
                    .entry(guild)
                    .or_insert_with(|| Chain::of_order(ORDER));
                markov.feed_str(x);
            }
            x if x.starts_with("!load") => {
                if msg.author.id.0 != 157_149_752_327_143_425 {
                    msg.channel_id.say(&ctx.http, "No permissions :)").unwrap();
                    msg.react(ctx.http, "👎").unwrap();
                    return;
                }

                let query = msg.content.as_str();
                let query = &query[query.find(' ').unwrap_or(4) + 1..];
                let (mut id, target) = if !query.is_empty() {
                    if let Some(x) = query.find(' ') {
                        let target = (&query[..x]).parse::<u64>().unwrap();
                        let id = (&query[x + 1..]).parse::<u64>().unwrap();
                        (MessageId(id), target)
                    } else {
                        (msg.id, query.parse::<u64>().unwrap())
                    }
                } else {
                    (msg.id, 1000)
                };
                let mut x = 0;

                while x < target {
                    let mut markov = self.markov.lock().unwrap();
                    let markov = markov
                        .entry(guild)
                        .or_insert_with(|| Chain::of_order(ORDER));

                    let res = channel
                        .messages(ctx.http.clone(), |x| x.before(id).limit(100))
                        .unwrap();

                    if res.is_empty() {
                        break;
                    }

                    for msg in res {
                        if !(msg.author.bot
                            || msg.content.starts_with('!')
                            || msg.content.starts_with(";;")
                            || msg.content.trim().is_empty())
                        {
                            markov.feed_str(&msg.content);
                            x += 1;
                            id = std::cmp::min(id, msg.id);
                            println!("{}, {}", x, id);
                        }
                    }
                }

                msg.channel_id
                    .say(&ctx.http, format!("Continue at {}", id))
                    .unwrap();
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
