use hashbrown::HashMap;

use rand::prelude::*;

use serde_derive::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Chain {
    map: HashMap<Option<Vec<String>>, (Vec<String>, Vec<usize>)>,
    single: HashMap<String, (Vec<String>, Vec<usize>)>,
    order: usize,
}

impl Chain {
    pub fn of_order(order: usize) -> Self {
        Self {
            map: HashMap::new(),
            single: HashMap::new(),
            order,
        }
    }

    pub fn feed(&mut self, tokens: &[String]) {
        fn handle(tokens: &mut Vec<String>, indices: &mut Vec<usize>, y: &String) {
            match tokens
                .iter()
                .enumerate()
                .find_map(|(i, v)| if v == y { Some(i) } else { None })
            {
                Some(i) => indices[i] += 1,
                None => {
                    tokens.push(y.clone());
                    indices.push(1);
                }
            }
        }

        if tokens.is_empty() {
            return;
        }

        let x = self.map.entry(None).or_insert_with(|| (vec![], vec![]));
        handle(&mut x.0, &mut x.1, &tokens[0]);

        for ts in tokens.windows(2) {
            let x = self
                .single
                .entry(ts[0].clone())
                .or_insert_with(|| (vec![], vec![]));
            handle(&mut x.0, &mut x.1, &ts[1]);
        }

        for ts in tokens.windows(self.order + 1) {
            let arr = ts[..self.order].to_vec();

            let x = self
                .map
                .entry(Some(arr))
                .or_insert_with(|| (vec![], vec![]));

            handle(&mut x.0, &mut x.1, &ts[self.order]);
        }
    }

    pub fn generate(&self, max: usize) -> Vec<String> {
        let mut rng = thread_rng();
        let actual_high = rng.gen_range(3, max);

        let mut ret = Vec::with_capacity(actual_high);

        let mut curr = &self.map[&None];

        for _ in 0..actual_high {
            for i in 0..100 {
                let dist = rand::distributions::WeightedIndex::new(&curr.1).unwrap();
                let choice = curr.0[dist.sample(&mut rng)].clone();

                if i < self.order {
                    curr = match self.single.get(&choice) {
                        Some(x) => x,
                        // None => &self.map[&None],
                        None => {
                            ret.push(choice);
                            break;
                        }
                    }
                } else {
                    let arr = ret[i - self.order..].to_vec();
                    curr = match self.map.get(&Some(arr)) {
                        Some(x) => x,
                        // None => &self.map[&None],
                        None => {
                            ret.push(choice);
                            break;
                        }
                    }
                };

                ret.push(choice);
            }
            ret.push("\n".to_owned());
        }

        ret
    }
}
