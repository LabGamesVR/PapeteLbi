use std::{
    borrow::Cow, io, sync::{
        mpsc::{self, Receiver, Sender, TryRecvError},
        Arc, Mutex,
    }, thread, time::{self, Duration}
};

use queue::Queue;

static TEMPO_NA_LISTA_NEGRA: u64 = 10;
static TEMPO_MAX_SEM_MSG: u64 = 3;
pub struct Comm {
    transmissores_fim: Vec<Sender<()>>,
    // filtro: Option<&'a (dyn Fn(&str) -> bool + Sync)>,
}

impl Comm {
    #[allow(dead_code)]
    pub fn new(queue: Arc<Mutex<Queue<String>>>) -> Self {
        let (tx1, rx1) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();
        let q1 = queue.clone();

        thread::spawn(move || Comm::buscador_portas(rx1, q1, None));
        thread::spawn(move || Comm::escutador_wifi(rx2, queue, None));

        Comm {
            transmissores_fim: vec![tx1, tx2],
        }
    }
    pub fn filtered(
        queue: Arc<Mutex<Queue<String>>>,
        filtro: &'static (dyn Fn(&str) -> bool + Sync),
    ) -> Self {
        let (tx1, rx1) = mpsc::channel();
        let (tx2, rx2) = mpsc::channel();
        let q1 = queue.clone();

        thread::spawn(move || Comm::buscador_portas(rx1, q1, Some(filtro)));
        thread::spawn(move || Comm::escutador_wifi(rx2, queue, Some(filtro)));

        Comm {
            transmissores_fim: vec![tx1, tx2],
        }
    }

    fn escutador_wifi(
        receptor_fim: Receiver<()>,
        queue: Arc<Mutex<Queue<String>>>,
        filtro: Option<&'static (dyn Fn(&str) -> bool + Sync)>,
    ) {
        use std::net::UdpSocket;

        let socket = match UdpSocket::bind("0.0.0.0:5555") {
            Ok(s) => s,
            Err(e) => panic!("couldn't bind socket: {}", e),
        };

        let mut buf = [0; 1000];
        loop {
            match receptor_fim.try_recv() {
                Ok(_) | Err(TryRecvError::Disconnected) => {
                    break;
                }
                Err(TryRecvError::Empty) => {}
            }

            match socket.recv_from(&mut buf) {
                Ok((_amt, _src)) => {
                    if let Ok(msg) = std::str::from_utf8(&buf) {
                        if let Ok(mut queue) = queue.lock() {
                            // println!("msg: {}",msg);
                            add_to_queue(&mut queue, msg, filtro);
                        }
                    }
                }
                Err(e) => {
                    println!("couldn't recieve a datagram: {}", e);
                }
            }
        }
    }

    pub fn portas_seriais_disponiveis() -> Vec<String> {
        let portas_diponiveis: Vec<String> = serialport::available_ports()
            .expect("erro ao ler portas")
            .iter()
            .map(|x| x.port_name.clone())
            .collect();
        return portas_diponiveis;
    }

    /*Enquanto não receber msg de fim:
    verifica se esta conectado.
    se não estiver, procura porta livre
    */
    fn buscador_portas(
        receptor_fim: Receiver<()>,
        queue: Arc<Mutex<Queue<String>>>,
        filtro: Option<&'static (dyn Fn(&str) -> bool + Sync)>,
    ) {
        let lista_negra: Arc<Mutex<Vec<(String, time::SystemTime)>>> =
            Arc::new(Mutex::new(Vec::new()));
        let portas_conectadas: Arc<Mutex<Vec<(String, Sender<()>)>>> =
            Arc::new(Mutex::new(Vec::new()));

        loop {
            match receptor_fim.try_recv() {
                Ok(_) | Err(TryRecvError::Disconnected) => {
                    break;
                }
                Err(TryRecvError::Empty) => {}
            }
            let disponiveis = Comm::portas_seriais_disponiveis();
            // println!("disponiveis: {:?}", disponiveis);

            if let Ok(mut portas) = portas_conectadas.lock() {
                if let Ok(mut lista_n) = lista_negra.lock() {
                    //retira da lista negra itens que estão lá a mais tempo que o necessario
                    lista_n.retain(|porta| match &porta.1.elapsed() {
                        Ok(p) => p.as_secs() < TEMPO_NA_LISTA_NEGRA,
                        Err(_e) => true,
                    });

                    for nome_porta in disponiveis {
                        if portas.iter().find(|pt| pt.0 == nome_porta).is_none()
                            && lista_n.iter().find(|pt| pt.0 == nome_porta).is_none()
                        {
                            let ref_a_lista = Arc::clone(&portas_conectadas);
                            let ref_a_lista_negra = Arc::clone(&lista_negra);
                            let ref_a_dados = Arc::clone(&queue);

                            let (tx, rx) = mpsc::channel();
                            portas.push((nome_porta.clone(), tx));

                            thread::spawn(move || {
                                println!("Tentar porta {}",nome_porta);
                                Comm::tentar_conexao_serial(
                                    nome_porta,
                                    ref_a_lista,
                                    ref_a_lista_negra,
                                    ref_a_dados,
                                    rx,
                                    filtro,
                                )
                            });
                        }
                    }
                }
            }
        }
    }

    fn tentar_conexao_serial(
        porta: String,
        lista: Arc<Mutex<Vec<(String, Sender<()>)>>>,
        lista_negra: Arc<Mutex<Vec<(String, time::SystemTime)>>>,
        dados: Arc<Mutex<Queue<String>>>,
        rx: Receiver<()>,
        filtro: Option<&'static (dyn Fn(&str) -> bool + Sync)>,
    ) {
        match serialport::new(Cow::from(&porta), 9600)
            .timeout(Duration::from_millis(60))
            .open()
        {
            Ok(porta_conectada) => {
                let f = filtro.clone();
                thread::spawn(move || {
                    Comm::individual_serial_listener(porta_conectada, rx, dados, lista, lista_negra, f)
                });
            }
            Err(_) => {
                if let Ok(mut l) = lista.lock() {
                    if let Some(index) = l.iter().position(|value| value.0 == porta) {
                        l.swap_remove(index);
                        if let Ok(mut ln) = lista_negra.lock(){
                            ln.push((porta, time::SystemTime::now()));
                        }
                    }
                }
            }
        }
    }

    fn individual_serial_listener(
        mut porta: Box<dyn serialport::SerialPort>,
        receptor_fim: Receiver<()>,
        queue: Arc<Mutex<Queue<String>>>,
        portas: Arc<Mutex<Vec<(String, Sender<()>)>>>,
        lista_negra: Arc<Mutex<Vec<(String, time::SystemTime)>>>,
        filtro: Option<&'static (dyn Fn(&str) -> bool + Sync)>,
    ) {
        println!("Ouvindo porta {}", porta.name().unwrap());
        let mut momento_ultima_mensagem = time::SystemTime::now();
        let mut serial_buf: Vec<u8> = vec![0; 10000];
        loop {
            if momento_ultima_mensagem.elapsed().unwrap().as_secs() > TEMPO_MAX_SEM_MSG{
                println!("Porta {} desconectada por timeout",porta.name().unwrap());
                if let Ok(mut ln) = lista_negra.lock(){
                    ln.push((porta.name().unwrap(), time::SystemTime::now()));
                }

                //localiza pos dessa conexao na lista
                let pos = portas
                .lock()
                .unwrap()
                .iter()
                .position(|r| &r.0 == &porta.name().unwrap())
                .unwrap();

                portas.lock().unwrap().swap_remove(pos);
                break;
            }
            match porta.read(serial_buf.as_mut_slice()) {
                Ok(t) => {
                    if let Ok(msg) = std::str::from_utf8(&serial_buf[..t]) {
                        if let Ok(mut queue) = queue.lock() {
                            if add_to_queue(&mut queue, msg, filtro){
                                momento_ultima_mensagem = time::SystemTime::now();
                            }
                        }
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
                Err(e) => {
                    println!("Desconectado por erro {}", e);

                    //localiza pos dessa conexao na lista
                    let pos = portas
                        .lock()
                        .unwrap()
                        .iter()
                        .position(|r| &r.0 == &porta.name().unwrap())
                        .unwrap();

                    portas.lock().unwrap().swap_remove(pos);
                    break;
                }
            }

            match receptor_fim.try_recv() {
                Ok(_) | Err(TryRecvError::Disconnected) => {
                    break;
                }
                Err(TryRecvError::Empty) => {}
            }
        }
    }
}

fn add_to_queue(
    queue: &mut Queue<String>,
    msg: &str,
    filtro: Option<&'static (dyn Fn(&str) -> bool + Sync)>,
) -> bool{
    //se o filtro aprova ou não tem filtro
    if if let Some(f) = filtro { f(msg) } else { true } {
        queue.queue(msg.to_owned()).unwrap();
        true
    }
    else {
        false
    }
}

//parar threads que possam estar executando
impl Drop for Comm {
    fn drop(&mut self) {
        for tx in &self.transmissores_fim {
            tx.send(()).unwrap();
        }
    }
}
