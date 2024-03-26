/*
O executavel serve para testes ou pra coleta de testes, como uma CLI.
O que vai fazer depende dos argumentos passados:
-coleta [-1 ou -2]
-teste -arvore
-teste -neural
-teste -neural -transferencia
-aval -arvore
-aval -neural
*/

mod arvore;
mod avaliacao;
mod conexao;
mod csv_helper;
mod dado_papete;
mod movimento;
mod neural;
mod papete;
mod previsor;

extern crate rand;
extern crate statistical;

use arvore::Arvore;
use dado_papete::DadoPapete;
use movimento::Movimento;
use neural::Neural;
use papete::Papete;
use previsor::Previsor;

use std::{
    io::{self, Write},
    thread, time,
};

/*
Teste da conexão USB: tenta conectar com as duas papetes.
Enquanto não achar nenhuma, aguarda; quando acha, imprime que achou e começa a
imprimir os dados obtidos de cada em loop.
*/
fn teste_serial() {
    let intervalo = time::Duration::from_millis(50);
    let mut papete = Papete::new();
    papete.ativar_modo_conexao_imediata(2);
    println!("Procurando papetes...");
    while papete.obter_conexoes().len() < 1 {
        thread::sleep(intervalo);
    }
    println!("Encontradas!");
    loop {
        let dados = papete.obter_dados();
        if dados.0.is_some() {
            print!("{}\t", dados.0.unwrap());
        }
        if dados.1.is_some() {
            print!("{}", dados.1.unwrap());
        }
        println!("");
    }
}

/*
Procura a qtd especificada de papetes (1 ou 2). Quando encontra, ela pede cada um dos movimentos
no numero especificado de rodadas. O usuario precisa confirmar estar pronto antes de cada movimento.
*/
fn coleta(papetes: usize, rodadas: usize) -> Vec<DadoPapete> {
    let intervalo = time::Duration::from_millis(80);

    let mut papete = Papete::new();
    papete.ativar_modo_conexao_imediata(papetes);
    println!("Procurando papetes...");
    while papete.obter_conexoes().len() < papetes {
        thread::sleep(intervalo);
    }
    thread::sleep(time::Duration::from_secs(1));
    println!("Encontradas!");
    papete.iniciar_sessao(500);

    for rodada in 0..rodadas {
        println!("{}/{}", rodada + 1, rodadas);
        for movimento in [
            Movimento::Repouso,
            Movimento::Dorsiflexao,
            Movimento::Flexao,
            Movimento::Eversao,
            Movimento::Inversao,
        ] {
            println!("{:?}", papete.obter_conexoes());
            println!("{}: {}", movimento.str_completa(), movimento.descricao());
            //aguarda input qqr
            io::stdin().read_line(&mut String::new()).unwrap();
            for _ in 0..10 {
                if papete.obter_conexoes().len() < papetes {
                    println!("Papete desconectou");
                    while papete.obter_conexoes().len() < papetes {
                        thread::sleep(intervalo);
                    }
                }
                papete.registrar(movimento);
                print!(".");
                io::stdout().flush().unwrap();
                thread::sleep(intervalo);
            }
            println!("");
            //println!("{:?}",papete.registrados);
        }
    }
    std::mem::replace(&mut papete.registrados, Vec::new())
}

/*
Conecta com a papete e imprime o que a arvore esta prevendo, em loop.
*/
fn teste_arvore() {
    let intervalo = time::Duration::from_millis(50);

    Arvore::calcular_de_dataset_addr("papete.csv")
        .unwrap()
        .salvar("arvore.JSON")
        .expect("Erro ao salvar arvore");
    let mut papete = Papete::new();
    papete.ativar_modo_conexao_imediata(1);
    print!("Procurando papetes...  ");
    while papete.obter_conexoes().len() < 1 {
        thread::sleep(intervalo);
    }
    thread::sleep(time::Duration::from_secs(1));
    println!("Encontradas!");
    papete.iniciar_sessao(0);

    loop {
        println!("{}", papete.obter_movimento());
        thread::sleep(intervalo);
    }
}

/*
Faz avaliação da arvore, com os dados de papete.csv
*/
fn aval_arvore() {
    avaliacao::teste_10_pastas::<Arvore>();
}

/*
Conecta com a papete e imprime o que a rede neural esta prevendo, em loop.
*/
fn teste_neural() {
    let mut n = Neural::carregar("papete.pt").unwrap();

    let dados = coleta(1, 2);
    n.transferir(&dados);

    let intervalo = time::Duration::from_millis(50);

    let mut papete = Papete::com_previsor(Box::new(n));
    papete.ativar_modo_conexao_imediata(1);
    print!("Procurando papetes...  ");
    while papete.obter_conexoes().len() < 1 {
        thread::sleep(intervalo);
    }
    thread::sleep(time::Duration::from_secs(1));
    println!("Encontradas!");
    papete.iniciar_sessao(0);

    loop {
        println!("{}", papete.obter_movimento());
        thread::sleep(intervalo);
    }
}

/*
Faz avaliação da rede neural, com os dados de papete.csv
*/
fn aval_neural() {
    avaliacao::teste_10_pastas::<Neural>();
}

/*
Faz uma coleta curta, a calibração do classificador e avaliação da rede neural,
com os dados de papete.csv.
*/
fn teste_transf() {
    println!("Primeiro precisamos de uma coleta");
    let dados = coleta(1, 1);
    let mut papete = Papete::com_previsor(Box::new(Neural::carregar("papete.pt").unwrap()));
    papete.ativar_modo_conexao_imediata(1);
    papete.transferir(&dados);
    loop {
        println!("{}", papete.obter_movimento());
    }
}

/*
Interpreta os argumentos e chama os procedimentos correspondentes
*/
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 1 {
        teste_serial();
    } else {
        if args[1] == "coleta" {
            let num = args.get(2).map(String::as_str).unwrap_or("1");
            let num = if num == "1" { 1 } else { 2 };
            let dados = coleta(num, 2);
            csv_helper::salvar_dados("papete.csv", &dados).unwrap();
        } else if args[1] == "teste" {
            let outro_arg = args.get(2).map(String::as_str).unwrap_or("arvore");
            if outro_arg == "arvore" {
                teste_arvore();
            } else if outro_arg == "neural" {
                teste_neural();
            } else if outro_arg.starts_with("trans") {
                teste_transf();
            } else {
                println!("argumento não reconhecido");
            }
        } else if args[1].starts_with("aval") {
            let outro_arg = args.get(2).map(String::as_str).unwrap_or("arvore");
            if outro_arg == "arvore" {
                aval_arvore();
            } else if outro_arg == "neural" {
                aval_neural();
            } else {
                println!("argumento não reconhecido");
            }
        } else {
            println!("argumento não reconhecido ({})",args[1]);
        }
    }
}
