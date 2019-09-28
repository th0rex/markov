use std::{
    env,
    sync::{Arc, Mutex},
};

use hashbrown::HashMap;

use serenity::{
    model::{
        channel::Message,
        gateway::{Activity, Ready},
        id::{GuildId, MessageId},
    },
    prelude::*,
};

mod markov;

use markov::Chain;

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

const DEFAULT_SIZE: usize = 10;
const ORDER: usize = 3;

struct Handler {
    markov: Arc<Mutex<HashMap<GuildId, Chain>>>,
}

fn feed(markov: &mut Chain, msg: &str) {
    let mut v = Vec::with_capacity(20);

    for x in msg.split('.') {
        v.clear();
        v.extend(x.split('.').map(|x| x.to_owned()));

        markov.feed(&v[..]);
    }
}

impl EventHandler for Handler {
    fn message(&self, ctx: Context, msg: Message) {
        let channel = msg.channel_id;
        let guild = match msg.guild_id {
            Some(x) => x,
            None => return,
        };

        if msg.author.bot {
            return;
        }

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

                let mut size = x
                    .find(' ')
                    .and_then(|i| (&x[i + 1..]).parse::<usize>().ok())
                    .unwrap_or(DEFAULT_SIZE);

                let mut s = markov.generate(size).join(" ");
                while s.bytes().len() > 2000 {
                    size /= 2;
                    s = markov.generate(size).join(" ");
                }

                if s.is_empty() || s.bytes().len() > 2000 {
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
                    let writer = zstd::Encoder::new(
                        std::fs::File::create(format!("./data/{}", k.0)).unwrap(),
                        3,
                    )
                    .unwrap()
                    .auto_finish();
                    bincode::serialize_into(writer, v).unwrap();
                }

                msg.react(ctx.http, "👍").unwrap();
            }
            x if !(msg.author.bot
                || x.starts_with('!')
                || x.starts_with(";;")
                || x.starts_with("=tex")) =>
            {
                let mut markov = self.markov.lock().unwrap();
                let markov = markov
                    .entry(guild)
                    .or_insert_with(|| Chain::of_order(ORDER));
                feed(markov, x);
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
                            || msg.content.starts_with("=tex")
                            || msg.content.trim().is_empty())
                        {
                            feed(markov, &msg.content);
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

            let reader = zstd::Decoder::new(std::fs::File::open(entry.path())?)?;

            markov.insert(GuildId(x), bincode::deserialize_from(reader).unwrap());
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
