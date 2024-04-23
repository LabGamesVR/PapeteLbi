pub mod arvore;
pub mod avaliacao;
pub mod conexao;
pub mod csv_helper;
pub mod dado_papete;
pub mod movimento;
pub mod neural;
pub mod papete;
pub mod previsor;

pub mod comm;

use std::ffi::CString;
use std::os::raw::c_char;

use conexao::Conexao;
use movimento::Movimento;
use neural::Neural;
use papete::Papete;
use previsor::Previsor;

extern crate simple_error;

#[no_mangle]
pub unsafe extern "C" fn alocar_papete() -> *mut Papete {
    use std::fs;
    use std::io::Write;
    use std::path::Path;

    //arquivo da rede neural
    let file_path = "papete.pt";
    //se não existe, cria
    if !Path::new(&file_path).exists() {
        fs::File::create(&file_path).unwrap_err_unchecked();
    }
    //inclui arquivo durante build
    let bytes = include_bytes!("..\\papete.pt");

    //escreve o arquivo incluido durante runtime
    match fs::OpenOptions::new().write(true).open(&file_path) {
        Ok(mut file) => {
            if let Err(err) = file.write_all(bytes) {
                eprintln!("Falha ao escrever em arquivo: {}", err);
            }
        }
        Err(err) => {
            eprintln!("Falha ao abrir arquivo: {}", err);
        }
    }

    //carrega rede neural
    let n = Neural::carregar("papete.pt").unwrap();

    //agora que já usou, exclui arquivo
    match fs::remove_file(&file_path) {
        Ok(_) => println!("File deleted: {}", file_path),
        Err(err) => eprintln!("Failed to delete file: {}", err),
    }

    let s = Box::new(Papete::com_previsor(Box::new(n)));
    Box::into_raw(s)
}

#[no_mangle]
pub unsafe extern "C" fn liberar_papete(s: *mut Papete) {
    drop(Box::from_raw(s));
}

#[no_mangle]
pub unsafe extern "C" fn iniciar_sessao(s: *mut Papete) {
    (*s).iniciar_sessao(0);
}

/*
Recebe um ptr de char, supõe que possui capacidade suficiente - uns 100 bytes são suficientes
Retorna quanto foi ocupado dela.
*/
#[no_mangle]
pub unsafe extern "C" fn listar_conexoes_disponiveis(array_ptr: *mut u8) -> i32 {
    let mut preenchidos = 0;
    let mut contador = 0;
    for c in Papete::listar_conexoes_disponiveis() {
        if let Conexao::USB(str) = c {
            let c_string = CString::new(str.as_str()).unwrap();
            for byte in c_string.as_bytes() {
                *(array_ptr.offset(preenchidos)) = *byte;
                preenchidos += 1;
            }
            *(array_ptr.offset(preenchidos)) = b',';
            preenchidos += 1;
            contador += 1;
        }
    }
    //null-terminator
    *(array_ptr.offset(preenchidos - 1)) = 0;
    contador
}

/*
Recebe um ptr de char, supõe que possui capacidade suficiente - uns 100 bytes são suficientes.
Retorna quanto foi ocupado dela.
*/
#[no_mangle]
pub unsafe extern "C" fn listar_conexoes_atuais(s: *mut Papete, array_ptr: *mut u8) -> i32 {
    let mut preenchidos = 0;
    let mut contador = 0;
    for c in (*s).obter_conexoes() {
        if let Conexao::USB(str) = c {
            let c_string = CString::new(str.as_str()).unwrap();
            for byte in c_string.as_bytes() {
                *(array_ptr.offset(preenchidos)) = *byte;
                preenchidos += 1;
            }
            *(array_ptr.offset(preenchidos)) = b',';
            preenchidos += 1;
            contador += 1;
        }
    }
    //null-terminator
    *(array_ptr.offset(preenchidos - 1)) = 0;
    contador
}

/*
Conecta na porta especificada
*/
#[no_mangle]
pub unsafe extern "C" fn conectar(_s: *mut Papete, _porta_chars: *const c_char) -> bool {
    println!("Inutilizado");
    true
}

#[no_mangle]
pub unsafe extern "C" fn obter_movimento(s: *mut Papete) -> Movimento {
    (*s).obter_movimento()
}

#[no_mangle]
pub unsafe extern "C" fn ativar_modo_conexao_imediata(s: *mut Papete) {
    (*s).ativar_modo_conexao_imediata(1);
}

#[no_mangle]
pub unsafe extern "C" fn desativar_modo_conexao_imediata(s: *mut Papete) {
    (*s).desativar_modo_conexao_imediata();
}

#[repr(C)]
pub struct Vec2 {
    x: f32,
    y: f32,
}

#[no_mangle]
pub unsafe extern "C" fn obter_rotacao(s: *mut Papete) -> Vec2 {
    match (*s).obter_dados_qqr() {
        Some(dado) => Vec2 {
            x: dado.pitch,
            y: dado.roll,
        },
        None => Vec2 { x: 0.0, y: 0.0 },
    }
}

#[no_mangle]
pub unsafe extern "C" fn obter_lado(s: *mut Papete) -> bool {
    match (*s).obter_dados_qqr() {
        Some(dado) => dado.lado_esq,
        None => false,
    }
}

#[no_mangle]
pub unsafe extern "C" fn registrar(s: *mut Papete, mov: Movimento) -> i32 {
    (*s).registrar(mov);
    (*s).registrados.len() as i32
}

#[no_mangle]
pub unsafe extern "C" fn deregistrar(s: *mut Papete) -> i32 {
    (*s).deregistrar();
    (*s).registrados.len() as i32
}

#[no_mangle]
pub unsafe extern "C" fn zerar(s: *mut Papete) -> i32 {
    (*s).registrados.clear();
    0
}

#[no_mangle]
pub unsafe extern "C" fn retreinar(s: *mut Papete) -> bool {
    if (*s).registrados.len() > 0 {
        (*s).transferir(&(*s).registrados);
        return true;
    }
    false
}
