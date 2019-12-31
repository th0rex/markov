use std::{
    env,
    io::{Read, Write},
    sync::{Arc, Mutex},
};

use hashbrown::HashMap;

use lazy_static::lazy_static;

use regex::Regex;

use serde::{Deserialize, Serialize};

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

const DEFAULT_SIZE: usize = 5;
const ORDER: usize = 3;

lazy_static! {
    static ref URL_REGEX: Regex = Regex::new("(https|http)://[^\\s]+").unwrap();
}

struct Handler {
    markov: Arc<Mutex<HashMap<GuildId, Chain>>>,
}

fn feed(markov: &mut Chain, msg: &str) {
    let mut v = Vec::with_capacity(20);
    let msg = URL_REGEX.replace_all(msg, "");

    for x in msg.split('.') {
        v.clear();
        v.extend(x.split(' ').map(|x| x.to_owned()));

        markov.feed(&v[..]);
    }
}

fn de<T: for<'a> Deserialize<'a>>(r: impl Read) -> T {
    bincode::deserialize_from(zstd::Decoder::new(r).unwrap()).unwrap()
}

fn se(w: impl Write, x: &impl Serialize) {
    bincode::serialize_into(zstd::Encoder::new(w, 3).unwrap().auto_finish(), x).unwrap()
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
            "!backup" => {
                let mut after_msg = 544495565950550016;
                let from_channel = 485850064485351432;
                let to_channel = 661619384263114752;

                let http = &ctx.http;
                let from_channel = http.get_channel(from_channel).unwrap().guild().unwrap();
                let to_channel = http.get_channel(to_channel).unwrap().guild().unwrap();
                let from_channel = from_channel.read();
                let to_channel = to_channel.read();

                loop {
                    let mut x = from_channel.messages(&ctx, |x| x.after(after_msg)).unwrap();
                    x.sort_by_key(|x| x.id.0);
                    for x in &x {
                        after_msg = std::cmp::max(after_msg, x.id.0);
                        println!("{} -- {} -- {}", x.id.0, x.attachments.len(), x.content);

                        let attachments = x
                            .attachments
                            .iter()
                            .map(|x| (x.download().unwrap(), x.filename.clone()))
                            .collect::<Vec<_>>();
                        to_channel
                            .send_message(&ctx, |mut f| {
                                f = f.content(x.content.clone());
                                for (a, b) in &attachments {
                                    f = f.add_file((a.as_slice(), b.as_ref()));
                                }
                                f
                            })
                            .unwrap();
                        std::thread::sleep_ms(500);
                    }

                    if x.len() < 50 {
                        break;
                    }
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
                    se(std::fs::File::create(format!("./data/{}", k.0)).unwrap(), v);
                }

                msg.react(ctx.http, "👍").unwrap();
            }
            "!load_raw" => {
                if msg.author.id.0 != 157_149_752_327_143_425 {
                    msg.channel_id.say(&ctx.http, "No permissions :)").unwrap();
                    msg.react(ctx.http, "👎").unwrap();
                    return;
                }

                let mut markov = self.markov.lock().unwrap();
                let markov = markov
                    .entry(guild)
                    .or_insert_with(|| Chain::of_order(ORDER));

                let x: Vec<String> = de(std::fs::File::open(format!("./raw/{}", guild.0)).unwrap());
                for x in x {
                    feed(markov, x.as_str());
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

                let mut all = Vec::with_capacity(target as usize);

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
                            all.push(msg.content.clone());
                            feed(markov, &msg.content);
                            x += 1;
                            id = std::cmp::min(id, msg.id);
                            println!("{}, {}", x, id);
                        }
                    }
                }

                let f = std::fs::File::create(format!("./raw/{}", guild.0)).unwrap();
                se(f, &all);

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

            markov.insert(GuildId(x), de(std::fs::File::open(entry.path())?));
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
