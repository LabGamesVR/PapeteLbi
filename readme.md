Esse projeto busca facilitar a implantação de jogos usando a papete inteligente. 

# script papete
na pasta arduino, é o que deve ser enviado para o arduino preso no pé.

# interface para jogos
o DLL "papete.dll" inclui funcoes para se conectar com a papete, ler o seu sensor e ler a posição atual (Dorsiflexão, Flexão plantar, Repouso, Eversão ou Inversão).

## Unity
Na pasta Assets/Plugins, precisa adicionar "papete.dll", bem como as outras dlls da qual ela depende (asmjit, c10, fbgemm, fbjini, libiomp5md, torch_cpu).
Depois, copiar "papete.cs" para o projeto, para poder usar a classe "Papete".

No exemplo "Carro Papete", é demonstrado isso.
> Atenção: como torch_cpu.dll é maior que 100MB, ele foi mandado para um zip. Basta descompactar para usar

# Rust

Para compilar somente o executavel (usado para treino e testes):

    cargo build --bin papete

Para compilar a biblioteca, para uso externo:

    cargo build --release --lib

Se não especifica --bin ou --lib, ocasionalmente link.exe resulta em erro