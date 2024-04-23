use crate::comm::comm::Comm;
use queue::Queue;
use std::{
    sync::{Arc, Mutex},
    thread, time,
};

static TIMEOUT: u64 = 1;
static LISTA_BRANCA: [&'static str; 4] = ["papE", "papD", "luvaE", "luvaD"];

#[derive(Debug)]
pub struct Sensor {
    pub device: String,
    pub values: Vec<f32>,
    pub time: time::SystemTime,
}

pub struct Sensores {
    pub sensores: Arc<Mutex<Vec<Sensor>>>,
    _comm: Comm,
}

fn filtro(msg: &str) -> bool {
    /*
    [a-zA-Z]: matches any single letter (uppercase or lowercase).
    [^\t]*: matches any number of symbols except for a tab character.
    \t: matches a tab character.
    .{3,}: matches at least three symbols.
    */
    let re = regex::Regex::new(r"^[a-z][^\t]*\t.{3,}").unwrap();
    if re.is_match(msg) {
        // println!("\"{}\" passou pelo filtro",msg);
        true
    } else {
        // println!("msg falha: \"{}\"",msg);
        false
    }
}

impl Sensores {
    pub fn new() -> Self {
        let queue = Arc::new(Mutex::new(Queue::new()));
        let comm = Comm::filtered(Arc::clone(&queue), &filtro);

        let sensores = Arc::new(Mutex::new(Vec::new()));
        let copia = Arc::clone(&sensores);

        thread::spawn(move || Sensores::listener(queue, copia));
        let s = Sensores {
            sensores,
            _comm: comm,
        };
        s
    }
    pub fn obter_sensores_ativos(&self) -> Vec<String> {
        self.sensores
            .lock()
            .unwrap()
            .iter()
            .filter(|s| s.time.elapsed().unwrap().as_secs() < TIMEOUT)
            .map(|s| s.device.to_string())
            .collect()
    }
    fn listener(queue: Arc<Mutex<Queue<String>>>, sensores: Arc<Mutex<Vec<Sensor>>>) {
        let re = regex::Regex::new(r"^-?(0|[1-9]\d*)(\.\d+)?").unwrap();
        let mut filtrou;
        loop {
            filtrou = false;
            if let Some(msg) = queue.lock().unwrap().dequeue() {
                // println!("msg: \"{}\"",msg.trim());
                let iterator = msg.trim().split('\t');
                if let Some(device) = iterator.clone().take(1).collect::<Vec<&str>>().get(0) {
                    if LISTA_BRANCA.contains(&device) {
                        let mut values: Vec<f32> = Vec::with_capacity(iterator.clone().count() - 1);
                        for item in iterator.skip(1) {
                            if let Ok(v) = re
                                .find(item)
                                .map(|x| x.as_str())
                                .unwrap_or("")
                                .parse::<f32>()
                            {
                                values.push(v);
                            }
                        }
                        if let Ok(mut s) = sensores.lock() {
                            if let Some(index) =
                                s.iter().position(|sensor| &sensor.device == device)
                            {
                                s[index].values = values;
                                s[index].time = time::SystemTime::now();
                            } else {
                                s.push(Sensor {
                                    device: device.to_string(),
                                    values,
                                    time: time::SystemTime::now(),
                                })
                            }

                            //retira da lista itens que estão a mais tempo que o necessario
                            s.retain(|sensor| match &sensor.time.elapsed() {
                                Ok(p) => p.as_secs() < TIMEOUT,
                                Err(_e) => true,
                            });
                            filtrou = true;
                            // println!("Filtrado");
                        }
                    }
                }
            }
            if !filtrou {
                if let Ok(mut s) = sensores.lock() {
                    //retira da lista itens que estão a mais tempo que o necessario
                    s.retain(|sensor| match &sensor.time.elapsed() {
                        Ok(p) => p.as_secs() < TIMEOUT,
                        Err(_e) => true,
                    });
                }
            }
        }
    }

    pub fn obter_valores(&self, buffer: &mut Vec<Vec<f32>>) {
        if let Ok(sensores) = self.sensores.lock() {
            //para cada um dos sensores
            for i in 0..sensores.len() {
                //garante que tem espaço para ele na lista
                if buffer.len() < i + 1 {
                    buffer.push(Vec::with_capacity(2));
                }
                //para cada valor do sensor
                for j in 0..sensores[i].values.len() {
                    if buffer[i].len() < j + 1 {
                        buffer[i].push(sensores[i].values[j]);
                    } else {
                        buffer[i][j] = sensores[i].values[j];
                    }
                }
                buffer[i].truncate(sensores[i].values.len());
            }
            buffer.truncate(sensores.len());
        }
    }
}
