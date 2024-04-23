/*
Pede lista de portas
Pede para conectar em COM4
pede dados
COM4 é desconectada
COM4 é reconectada
Precisa pedir de novo para conectar
*/

use crate::{
    comm::sensor::Sensores, conexao::Conexao, csv_helper, dado_papete::DadoPapete,
    movimento::Movimento, neural::Neural, previsor::Previsor,
};

use std::time::{SystemTime, UNIX_EPOCH};

pub struct Papete {
    offsets: (Option<DadoPapete>, Option<DadoPapete>),
    previsor: Option<Box<dyn Previsor>>,
    pub registrados: Vec<DadoPapete>,
    sessao: Option<u32>,
    sensores: Sensores,
}

impl Papete {
    pub fn new() -> Papete {
        Papete {
            offsets: (None, None),
            previsor: None,
            registrados: Vec::new(),
            sessao: None,
            sensores: Sensores::new(),
        }
    }

    pub fn com_previsor(previsor: Box<dyn Previsor>) -> Papete {
        Papete {
            offsets: (None, None),
            previsor: Some(previsor),
            registrados: Vec::new(),
            sessao: None,
            sensores: Sensores::new(),
        }
    }

    pub fn obter_movimento(&mut self) -> Movimento {
        let nomes_sensores = self.sensores.obter_sensores_ativos();
        let mut buffer: Vec<Vec<f32>> = Vec::with_capacity(2);
        self.sensores.obter_valores(&mut buffer);
        for i in 0..buffer.len() {
            if let Some(pitch) = buffer[i].get(0) {
                if let Some(roll) = buffer[i].get(1) {
                    let pe_esq = nomes_sensores[i] == "papE";
                    let mut dado = DadoPapete::basico(pitch.clone(), roll.clone(), pe_esq);
                    
                    if pe_esq{
                        if let Some(offset) = self.offsets.0 {
                            dado -= offset;
                            return self.previsor.as_mut().unwrap().prever(dado);
                        } else {
                            self.offsets.0 = Some(dado);
                            println!("Coloquei offset 0");
                        }
                    }
                    else {
                        if let Some(offset) = self.offsets.1 {
                            dado -= offset;
                            return self.previsor.as_mut().unwrap().prever(dado);
                        } else {
                            self.offsets.1 = Some(dado);
                            println!("Coloquei offset 1");
                        }
                    }
                }
            }
        }
        println!("Não consegui papete");
        Movimento::Repouso
    }

    pub fn obter_conexoes(&self) -> Vec<Conexao> {
        self.sensores
            .obter_sensores_ativos()
            .iter()
            .map(|sensor| Conexao::USB(sensor.clone()))
            .collect()
    }

    pub fn obter_dados(&self) -> (Option<DadoPapete>, Option<DadoPapete>) {
        let mut dados = (None, None);

        let nomes_sensores = self.sensores.obter_sensores_ativos();
        let mut buffer: Vec<Vec<f32>> = Vec::with_capacity(2);
        self.sensores.obter_valores(&mut buffer);
        for i in 0..buffer.len() {
            if let Some(pitch) = buffer[i].get(0) {
                if let Some(roll) = buffer[i].get(1) {
                    let pe_esq = nomes_sensores[i] == "papE";
                    let dado = DadoPapete::basico(pitch.clone(), roll.clone(), pe_esq);
                    if pe_esq {
                        //possivel ponto de erro! Não sei se é 1 ou 0
                        dados.1 = Some(dado);
                    } else {
                        dados.0 = Some(dado);
                    }
                }
            }
        }
        return dados;
    }

    #[allow(dead_code)]
    pub fn obter_dados_qqr(&self) -> Option<DadoPapete> {
        let nomes_sensores = self.sensores.obter_sensores_ativos();
        let mut buffer: Vec<Vec<f32>> = Vec::with_capacity(2);
        self.sensores.obter_valores(&mut buffer);
        for i in 0..buffer.len() {
            if let Some(pitch) = buffer[i].get(0) {
                if let Some(roll) = buffer[i].get(1) {
                    let pe_esq = nomes_sensores[i] == "papE";
                    let dado = DadoPapete::basico(pitch.clone(), roll.clone(), pe_esq);
                    return Some(dado);
                }
            }
        }
        None
    }

    #[allow(dead_code)]
    pub fn listar_conexoes_disponiveis() -> Vec<Conexao> {
        let portas_diponiveis: Vec<Conexao> = serialport::available_ports()
            .expect("erro ao ler portas")
            .iter()
            .map(|x| Conexao::USB(x.port_name.clone()))
            .collect();
        return portas_diponiveis;
    }

    pub fn iniciar_sessao(&mut self, qtd_esperada: usize) {
        self.registrados = Vec::with_capacity(qtd_esperada);
        self.offsets = self.obter_dados();
        self.sessao = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u32,
        );
    }
    pub fn registrar(&mut self, movimento: Movimento) -> bool {
        let dados = self.obter_dados();
        let mut res = false;
        for lado in [
            (dados.0, &mut self.offsets.0),
            (dados.1, &mut self.offsets.1),
        ] {
            if let Some(mut x) = lado.0 {
                if let Some(offset) = lado.1 {
                    x.movimento = Some(movimento);
                    x.sessao = Some(if let Some(sessao) = self.sessao {
                        sessao
                    } else {
                        0
                    });
                    x -= *offset;
                    self.registrados.push(x);
                    res = true;
                } else if movimento == Movimento::Repouso {
                    *lado.1 = lado.0.clone();
                }
            }
        }
        res
    }
    #[allow(dead_code)]
    pub fn deregistrar(&mut self) {
        self.registrados.pop();
    }
    #[allow(dead_code)]
    pub fn salvar(&mut self, destino: &str) -> std::io::Result<()> {
        csv_helper::salvar_dados(destino, &self.registrados)
    }

    pub fn ativar_modo_conexao_imediata(&mut self, _max_conexoes: usize) {
        println!("Inultilizado");
    }
    #[allow(dead_code)]
    pub fn desativar_modo_conexao_imediata(&mut self) {
        println!("Inultilizado");
    }
}

impl Previsor for Papete {
    fn calcular_de_dataset(dataset: &[DadoPapete]) -> Result<Self, Box<dyn std::error::Error>> {
        match Neural::calcular_de_dataset(dataset) {
            Ok(n) => Ok(Papete::com_previsor(Box::new(n))),
            Err(e) => Err(e),
        }
    }
    fn carregar(endereco: &str) -> Result<Self, Box<dyn std::error::Error>> {
        match Neural::carregar(endereco) {
            Ok(n) => Ok(Papete::com_previsor(Box::new(n))),
            Err(e) => Err(e),
        }
    }
    fn salvar(&self, endereco: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.previsor.as_ref().unwrap().salvar(endereco)
    }

    fn prever(&mut self, entrada: DadoPapete) -> Movimento {
        self.previsor.as_mut().unwrap().prever(entrada)
    }
    fn prever_batch(&mut self, entrada: &[DadoPapete]) -> Vec<Movimento> {
        self.previsor.as_mut().unwrap().prever_batch(entrada)
    }
    fn transferir(&mut self, dataset: &[DadoPapete]) {
        self.previsor.as_mut().unwrap().transferir(dataset)
    }
}
