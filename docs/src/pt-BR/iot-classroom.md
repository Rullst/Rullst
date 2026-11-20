# Robótica e IoT em sala de aula

> **Tradução** da página em inglês [`iot-classroom.md`](../iot-classroom.md), feita a partir do commit [`09efdd525`](https://github.com/Rullst/Rullst/commit/09efdd5257db6a8b8311ea62098a7f8c4b7a25cf). Se as duas versões divergirem, vale a versão em inglês.

Este guia é para professores que querem combinar placas de robótica com uma
aplicação web Rullst em sala de aula: quais placas escolher, ideias de
projetos e notas de segurança. Ele mantém separados três tipos de afirmação:

- **Validado com o Rullst:** o que o projeto já executou em hardware real,
  com a data e um link para os resultados. Só o BBC micro:bit V2 e o Arduino
  Uno estão neste grupo.
- **Dados das placas:** conectividade e suporte a Rust, tirados das páginas
  dos fabricantes e da documentação dos crates em 2026-11-19 (veja as
  [fontes](#fontes-dos-dados-das-placas)).
- **Ideias:** todos os projetos abaixo. Nenhum deles é um exemplo publicado
  nem um plano de aula testado. Cada um diz quais partes do Rullst usaria e o
  que você mesmo constrói ou providencia.

## O que o Rullst traz para uma aula de robótica

As placas fazem o trabalho físico. O Rullst roda em um notebook ou em um
pequeno computador Linux e dá à turma uma página web: botões, leituras ao
vivo, histórico, um placar. Estas são as partes que as ideias abaixo usam:

| Parte | O que faz | Documentação |
| :--- | :--- | :--- |
| `rullst-iot` | `SensorTelemetry` (uma leitura: `device_id`, `metric`, `value` e `timestamp` em segundos UNIX inteiros), `AnomalyDetector`, codificadores de MQTT 5 PUBLISH e de CoAP, e a verificação de firmware assinado com Ed25519 (`OtaManager`). Roda no servidor e, sem a feature `std`, em uma placa. Não tem código de rede. | [Guia do rullst-iot](../crates/iot.md) |
| `rullst::iot_ingest` (feature `iot-ingest` da fachada) | `TelemetryReceiver`: uma rota HTTP que aceita JSON de `SensorTelemetry` com um token bearer por dispositivo e publica cada lote aceito em um canal de tempo real (`iot.telemetry` por padrão). | [Recebendo telemetria](../crates/iot.md#receiving-telemetry-on-the-server-rullstiot_ingest) |
| Tempo real do Core | `BroadcastManager` (canais), `PresenceTracker` (quem está conectado) e `RealtimeSse` (Server-Sent Events para uma página que usa a extensão SSE do htmx). | [Tutorial 34](../tutorials/34-realtime-dashboard-sse.md), [tutorial 17](../tutorials/17-websockets-realtime-chat.md) |
| ORM | Modelos com `#[derive(Orm)]` e migrações, em SQLite com as features `orm` e `strict-sqlite` da fachada. | [Tutorial 03](tutorials/03-active-record-crud.md) |
| CLI | `cargo rullst make:iot`, `iot:keygen`, `iot:sign` e `make:model --migration`. | [Referência da CLI](../cli_reference.md#cargo-rullst-iotkeygen) |
| `rullst-labs` | Contratos de exercício, submissão e correção exata. Você providencia o executor (*runner*) que roda o código dos alunos. | [Contrato do executor](../labs-runner-contract.md) |

O firmware de uma placa não precisa ser escrito em Rust. Na execução validada
com duas placas, as placas Arduino Uno rodavam um sketch Arduino em C++, e um
programa *gateway* no computador lia a porta serial USB de cada placa e
enviava as leituras ao Rullst. Esse programa é o
[`examples/iot-gateway`](https://github.com/Rullst/Rullst/tree/main/examples/iot-gateway),
o código funcional mais próximo de todas as ideias desta página: tokens por
placa, um registro de dispositivos em SQLite com uma página no Nexus e um
painel ao vivo com Server-Sent Events. Ele serve os próprios arquivos do htmx,
então sua página não precisa de internet. Nada nesse padrão depende da
linguagem de programação da placa, então um micro:bit programado com MakeCode
ou Python poderia alimentar o mesmo tipo de gateway; essa combinação não foi
testada.

## Placas

Os níveis de custo são relativos e aproximados, não preços: consulte as lojas
da sua região.

| Placa | Conectividade | Suporte a Rust | Validada com o Rullst | Custo relativo |
| :--- | :--- | :--- | :--- | :--- |
| **BBC micro:bit V2** | Rádio do micro:bit e Bluetooth Low Energy; sem Wi-Fi; serial USB até um computador | Crates da comunidade: crate de placa [`microbit-v2`](https://docs.rs/microbit-v2), [`embassy-nrf`](https://docs.rs/embassy-nrf) | **Sim:** o `rullst-iot` rodou na placa (2026-11-10) | Baixo |
| **Arduino Uno R3** | Nenhuma na placa; serial USB até um computador | Crate da comunidade: [`avr-hal`](https://github.com/Rahix/avr-hal) (`arduino-hal`), Rust nightly | **Sim, como placa de sensor:** um sketch Arduino alimentou um gateway Rullst (2026-11-10 e 11); o `rullst-iot` rodou no computador | Baixo |
| **ESP32-C3** | Wi-Fi de 2,4 GHz (802.11 b/g/n) e Bluetooth 5 (LE) | [`esp-hal`](https://docs.espressif.com/projects/rust/), com apoio da Espressif | **Não**, aguardando uma placa ([#637](https://github.com/Rullst/Rullst/issues/637)) | Muito baixo |
| **Raspberry Pi Pico 2 W** | Wi-Fi de 2,4 GHz (802.11n) e Bluetooth 5.2 | Crates da comunidade: [`embassy-rp`](https://docs.rs/embassy-rp), [`rp235x-hal`](https://docs.rs/rp235x-hal), [`cyw43`](https://docs.rs/cyw43) para o rádio | **Não** | Muito baixo |
| **ESP32-S3** (inclusive placas com câmera) | Wi-Fi de 2,4 GHz (802.11 b/g/n) e Bluetooth 5 (LE) | [`esp-hal`](https://docs.espressif.com/projects/rust/), com o fork do compilador da Espressif para Xtensa | **Não** | Baixo a médio |
| **Raspberry Pi Zero 2 W** | Wi-Fi de 2,4 GHz (802.11 b/g/n) e Bluetooth 4.2 (BLE) | Um computador Linux: Rust padrão (`aarch64-unknown-linux-gnu`), crate da comunidade [`rppal`](https://docs.rs/rppal) para os pinos | **Não:** o Rullst nunca rodou em nenhum Raspberry Pi | Baixo para a placa; médio com o cartão microSD, a fonte e os cabos de que ela precisa |

### BBC micro:bit V2

O micro:bit V2 (nRF52833: Arm Cortex-M4 com FPU a 64 MHz, 512 KB de flash,
128 KB de RAM) tem acelerômetro e magnetômetro, sensor de luz por meio do
display de LEDs 5×5, um sensor de temperatura dentro do processador,
microfone, alto-falante, dois botões e um logotipo sensível ao toque. Ele se
comunica com outros micro:bits pelo próprio protocolo de rádio de 2,4 GHz,
que o site técnico do micro:bit descreve como sem criptografia, e com outros
dispositivos por Bluetooth Low Energy. Não tem Wi-Fi. Para chegar a um
servidor, um segundo micro:bit ligado ao computador por USB pode servir de
ponte: ele recebe as mensagens de rádio e as imprime na sua porta serial USB,
onde um programa gateway as lê. Essa ponte é um desenho para a sua turma, não
algo que o Rullst testou. O micro:bit é muito amigável para iniciantes: blocos
do MakeCode ou Python, e muitos kits de robô montados em torno dele (veja a
página de [acessórios](https://microbit.org/buy/accessories/) da micro:bit
Foundation e as extensões de robótica da
[galeria do MakeCode](https://makecode.microbit.org/extensions/extension-gallery)).
Em Rust, o crate `microbit-v2` e o `embassy-nrf`, ambos da comunidade, dão
suporte a ele, e o
[livro Discovery](https://docs.rust-embedded.org/discovery-mb2/) do Rust
Embedded Working Group ensina com ele.

**Validado com o Rullst.** Em 2026-11-10 um micro:bit V2 rodou o próprio
`rullst-iot` (`default-features = false`, com o crate `microbit-v2` 0.16): 66
verificações de JSON de telemetria, codificação MQTT e CoAP, o detector de
anomalias, a verificação de firmware assinado (inclusive uma atualização
assinada com `cargo rullst iot:sign`) e o contrato do contador de rollback,
sem nenhuma falha
([resultados](../crates/iot.md#hardware-validation-bbc-microbit-v2)). A placa
informou os resultados pela serial USB; o rádio, o Bluetooth e a ponte acima
não fizeram parte dessa execução.

### Arduino Uno R3

O Uno R3 tem um ATmega328P a 16 MHz com 32 KB de flash e 2 KB de SRAM, 14
pinos digitais (6 com PWM) e 6 entradas analógicas. Não tem rádio e chega a
um computador pela serial USB. Normalmente é programado em C++ na IDE do
Arduino. Em Rust, o `avr-hal` (`arduino-hal`), da comunidade, dá suporte a ele
com um compilador nightly, e o alvo AVR do Rust é Tier 3 (não é compilado nem
testado automaticamente pelo projeto Rust). O Uno R4 WiFi é outra placa, com
um Renesas RA4M1 e um módulo ESP32-S3 para Wi-Fi e Bluetooth, que o Rullst
não testou.

**Validado com o Rullst, como placa de sensor.** Em 2026-11-10 as leituras de
um Uno passaram por `SensorTelemetry`, pelo codificador MQTT e pelo
`AnomalyDetector` em um gateway Rullst no computador. Em 2026-11-11 duas
placas compatíveis com o Uno (chip USB CH340) enviaram leituras ao mesmo
tempo, cada uma pelo seu gateway, para o `rullst::iot_ingest`, e depois que
uma placa foi revogada todo envio posterior com o token dela foi recusado
([execução com duas placas](../crates/iot.md#two-boards-through-rullstiot_ingest)).
As placas rodavam um sketch Arduino e o `rullst-iot` rodava no computador: com
2 KB de RAM, o Uno não comporta os cerca de 8 KiB de pilha de que a
verificação de assinatura do `rullst-iot` precisa. Nenhum broker ou rádio foi
usado.

### ESP32-C3

O ESP32-C3 tem um núcleo RISC-V de 32 bits de até 160 MHz, 400 KB de SRAM,
Wi-Fi de 2,4 GHz (802.11 b/g/n) e Bluetooth 5 (LE), em placas de
desenvolvimento de baixo custo. A Espressif apoia o suporte a Rust: o
`esp-hal` 1.0 (outubro de 2025) foi anunciado como o primeiro SDK Rust com
apoio de fabricante, com GPIO, UART, SPI e I2C na API estável e os demais
drivers atrás da feature `unstable`. Wi-Fi e Bluetooth passam pelo
`esp-radio`, que ainda não tinha chegado à 1.0 em 2026-11-19. O C3 compila com
Rust estável para `riscv32imc-unknown-none-elf`, e o
[treinamento no_std](https://docs.espressif.com/projects/rust/no_std-training/)
da Espressif foi escrito para ele. Com Wi-Fi, um C3 poderia enviar JSON de
`SensorTelemetry` ao `TelemetryReceiver` sem gateway, usando um cliente HTTP
de sua escolha; ele também precisaria saber a hora certa, porque o receptor
recusa, por padrão, timestamps mais de 5 minutos adiantados ou com mais de 24
horas.

**Ainda não validado com o `rullst-iot`.** Rodar as verificações do micro:bit
em um ESP32-C3 é uma candidata da 13.x que aguarda uma placa
([#637](https://github.com/Rullst/Rullst/issues/637)). A CI compila o
`rullst-iot` para `riscv32imac-unknown-none-elf`, um alvo RISC-V parecido, mas
não para o alvo do próprio C3, `riscv32imc-unknown-none-elf`, e compilar não é
um teste em hardware.

### Raspberry Pi Pico 2 W

O Pico 2 W tem um RP2350 (dois núcleos Arm Cortex-M33 ou dois RISC-V Hazard3 a
150 MHz, 520 KB de SRAM) com 4 MB de flash, e um Infineon CYW43439 para Wi-Fi
de 2,4 GHz (802.11n) e Bluetooth 5.2. Suas 12 máquinas de estado PIO são
pequenos processadores de E/S programáveis. Em Rust, o `embassy-rp`, da
comunidade, dá suporte ao RP2040 e ao RP235x e traz programas PIO prontos para
PWM, encoders de quadratura e motores de passo (`pio_programs`), que servem
para servos, encoders de roda e motores de passo; o `rp235x-hal`, do projeto
rp-rs, é o outro HAL, e o driver `cyw43` lista o Pico 2 W como funcionando
para Wi-Fi e Bluetooth. Os núcleos Arm usam o alvo
`thumbv8m.main-none-eabihf`.

**Não validado.** Nenhuma placa rodou o `rullst-iot`, e a CI não o compila
para `thumbv8m.main-none-eabihf`.

### ESP32-S3 e placas com câmera

O ESP32-S3 tem dois núcleos Xtensa LX7 de até 240 MHz, 512 KB de SRAM, Wi-Fi
de 2,4 GHz (802.11 b/g/n), Bluetooth 5 (LE) e instruções vetoriais para redes
neurais e processamento de sinais. A placa ESP32-S3-EYE da Espressif
acrescenta uma câmera de 2 MP, um LCD, um microfone, 8 MB de flash e 8 MB de
PSRAM, para exercícios simples de visão, como encontrar um marcador colorido.
O `esp-hal` dá suporte ao S3, mas o Xtensa exige o fork do compilador Rust da
Espressif, instalado com o `espup`. Para câmeras, o `esp-hal` tem uma
interface de câmera DVP (`lcd_cam::cam`) atrás da feature `unstable`, e
configurar o sensor da câmera fica a cargo do seu código; o driver de câmera
da Espressif, o [`esp32-camera`](https://github.com/espressif/esp32-camera),
é um componente em C para o ESP-IDF. O `TelemetryReceiver` só aceita leituras
de `SensorTelemetry`, então enviar imagens a um servidor Rullst exige uma rota
de upload sua.

**Não validado.**

### Raspberry Pi Zero 2 W

O Zero 2 W é um pequeno computador Linux, não um microcontrolador: um Arm
Cortex-A53 de quatro núcleos e 64 bits a 1 GHz, 512 MB de RAM, Wi-Fi de
2,4 GHz (802.11 b/g/n) e Bluetooth 4.2 com BLE. Ele roda o Raspberry Pi OS de
64 bits, então poderia rodar um servidor Rullst no próprio robô, com o painel
web e o controle dos motores na mesma máquina (não testado, veja abaixo).
`aarch64-unknown-linux-gnu` é um alvo Tier 1 do Rust, e o `rppal` acessa seus
GPIO, I2C, PWM, SPI e UART. Compile em um notebook e copie o binário: 512 MB é
pouca memória para compilar uma aplicação Rullst. O Linux não garante tempo
como um microcontrolador garante, por isso pulsos precisos (servos, encoders)
costumam ficar com uma placa microcontroladora ligada pela serial.

**Não validado.** O Rullst nunca rodou em nenhum Raspberry Pi. Só como
referência: o build de release do starter Blank usou cerca de 10 MB de memória
residente em repouso em um runner de CI Linux x86_64
([arquivo de limites](https://github.com/Rullst/Rullst/blob/main/.github/starter-footprint/blank.toml));
ninguém mediu em ARM.

## Ideias de projetos

Estas são ideias, não exemplos publicados. Cada uma nomeia as partes do
Rullst que usaria (conferidas no repositório em 2026-11-19), o que você
constrói ou providencia, o que os alunos aprendem e um nível de dificuldade.

### 1. Estação meteorológica da turma

- **Objetivo:** estações espalhadas pela sala, ou do lado de fora de uma
  janela, informam temperatura, luz e nível de som; uma página mostra os
  valores ao vivo e um gráfico do dia.
- **Placas:** estações micro:bit V2 (temperatura, luz e microfone já vêm na
  placa) com um micro:bit ligado por USB como ponte de rádio; ou placas Uno
  com sensores pela serial USB; ou um ESP32-C3 ou Pico 2 W enviando por Wi-Fi.
- **Partes do Rullst:** `TelemetryReceiver` (feature `iot-ingest`) com um
  token por estação e uma tabela de `DeviceDirectory` (copie o
  `src/registry.rs` do exemplo); `BroadcastManager` e `RealtimeSse` para
  levar as leituras à página, como faz o painel do exemplo; um modelo do ORM
  para o histórico (`cargo rullst make:model Reading --migration`, features
  `orm` e `strict-sqlite`), gravado por uma tarefa inscrita no canal
  `iot.telemetry`; `AnomalyDetector` para marcar leituras fora de uma faixa
  (`normal`, `warning`, `critical anomaly`).
- **Você constrói ou providencia:** o firmware das estações e da ponte; a
  tarefa que salva as leituras (o exemplo mantém a tabela de histórico só na
  memória); o gráfico, já que o Rullst não tem componente de gráfico: desenhe
  uma linha SVG em uma view `html!` ou adicione uma biblioteca de gráficos de
  sua escolha.
- **Os alunos aprendem:** sensores e unidades, amostragem, calibração (compare
  a temperatura do processador do micro:bit com um termômetro), séries
  temporais, bancos de dados e como uma página se atualiza ao vivo.
- **Dificuldade:** de iniciante a intermediária, partindo do exemplo.

O roadmap registra uma ideia relacionada, um
[exemplo de IoT offline](../../../ROADMAP.md#ideas-recorded-for-after-130)
com leituras em SQLite e um painel sem internet.

### 2. Robô com painel de controle web

- **Objetivo:** dirigir um pequeno robô pelo navegador de um celular ou
  notebook (frente, ré, esquerda, direita, parar) e acompanhar ao vivo a
  tensão da bateria e o sensor de distância.
- **Placas:** o Rullst em um notebook, com a placa dos motores (um Uno, ou um
  micro:bit em um kit de robô) em um cabo USB ou atrás da ponte de rádio do
  micro:bit; ou o Rullst em um Pi Zero 2 W montado no robô, falando com a
  placa dos motores pela serial ou acionando um driver de motor com o
  `rppal`; ou uma placa de motores ESP32-C3 ou Pico 2 W no Wi-Fi.
- **Partes do Rullst:** uma página `html!` cujos botões enviam requisições
  `hx-post`: os comandos mudam o estado do robô, por isso são requisições POST
  com o token CSRF
  ([contrato de CSRF dos formulários](../tutorials/07-forms-and-validation.md#step-3-send-the-csrf-token-with-browser-forms)),
  nunca GET; `BroadcastManager` e `RealtimeSse` para a telemetria, e
  `TelemetryReceiver` se ela chegar por HTTP; `cargo rullst auth` para
  colocar o painel atrás de um login. Em desenvolvimento o servidor só escuta
  em `127.0.0.1`; defina `HOST=0.0.0.0` para que os celulares na rede da sala
  possam abri-lo.
- **Você constrói ou providencia:** o caminho dos comandos até a placa dos
  motores, que o Rullst não oferece (o `iot_ingest` só recebe leituras). Pela
  serial USB, o crate [`serialport`](https://crates.io/crates/serialport),
  com que o gateway do exemplo lê, também escreve; ele bloqueia, então dê a
  ele uma thread própria ou use `spawn_blocking` em vez de chamá-lo dentro de
  um handler de requisição. Pelo Wi-Fi, a placa precisa do próprio cliente
  HTTP ou MQTT, e o MQTT também precisa de um broker (por exemplo o Eclipse
  Mosquitto) e de um crate cliente como o
  [`rumqttc`](https://crates.io/crates/rumqttc) no servidor. O firmware dos
  motores precisa parar quando os comandos param de chegar (veja as notas de
  segurança abaixo).
- **Os alunos aprendem:** cliente e servidor, por que comandos usam POST,
  latência, protocolos seriais, drivers de motor e PWM, e projeto à prova de
  falhas.
- **Dificuldade:** intermediária; avançada no Pi Zero 2 W, que exige
  compilação cruzada.

### 3. Placar de corrida de seguidores de linha

- **Objetivo:** cronometrar cada robô entre um portão de largada e um de
  chegada e mostrar um placar da turma ao vivo no projetor e nos celulares.
- **Placas:** um micro:bit ou Uno ligado aos sensores dos dois portões
  (sensores de feixe ou refletivos), para que um único relógio meça a volta
  inteira; ele informa a volta pela serial USB ou pela ponte de rádio do
  micro:bit.
- **Partes do Rullst:** `TelemetryReceiver` recebendo uma leitura `lap_ms`;
  `BroadcastManager` e `RealtimeSse` para atualizar o placar;
  `PresenceTracker` para mostrar quantas telas acompanham a corrida (um
  `user_joined` por página aberta, um `user_left` quando ela fecha,
  `count_online`); um modelo do ORM para as voltas
  (`cargo rullst make:model Lap --migration`).
- **Você constrói ou providencia:** os sensores dos portões e o firmware
  deles, uma página do professor para escolher qual equipe está na pista, e as
  regras de classificação. Os timestamps de `SensorTelemetry` são segundos
  inteiros, então a placa mede a volta e envia os milissegundos como valor da
  leitura, em vez de dois timestamps.
- **Os alunos aprendem:** relógios e medição de tempo (por que um único
  relógio mede a volta), debounce de sensores, ordenação, regras justas e
  páginas em tempo real.
- **Dificuldade:** intermediária.

O roadmap também registra uma
[ideia de quiz multijogador](../../../ROADMAP.md#ideas-recorded-for-after-130)
construída sobre `BroadcastManager` e `PresenceTracker`.

### 4. Atualizações de firmware assinadas

- **Objetivo:** mostrar por que um dispositivo verifica uma assinatura antes
  de instalar firmware. O professor assina uma imagem, a placa a aceita, e
  depois uma cópia com um byte alterado, uma versão alterada ou outra chave é
  recusada.
- **Placas:** micro:bit V2, a única placa em que isso já rodou.
- **Partes do Rullst:** `cargo rullst iot:keygen` (um par de chaves Ed25519:
  a chave secreta com modo `0600` no Unix, a chave pública como `.pub.rs`,
  `.pub.bin` e `.pub`) e `cargo rullst iot:sign` (o manifesto e sua
  assinatura, conferidos com `OtaManager::begin_update` antes de serem
  gravados); na placa, `OtaManifest::from_signing_bytes`,
  `OtaManager::begin_update` e o `OtaImageVerifier` que ele devolve, com um
  `RollbackCounterStore` para o contador anti-rollback. O firmware de teste do
  micro:bit no repositório,
  [`.github/fixtures/iot-microbit-v2`](https://github.com/Rullst/Rullst/tree/main/.github/fixtures/iot-microbit-v2),
  já roda essa demonstração quando compilado com `RULLST_IOT_SIGNED_DIR`
  apontando para a pasta com os arquivos abaixo, e rodou em 2026-11-10
  ([resultados](../crates/iot.md#cli-signed-update-on-the-board)). O
  `firmware/src/main.rs` dele explica como gravá-lo na placa e ler as linhas
  `PASS` e `FAIL` pela porta serial USB.

  ```console
  cargo rullst iot:keygen --out ota.key
  cargo rullst iot:sign --key ota.key --image image.bin --target microbit-v2 \
    --version 2.0.0 --counter 7
  ```

- **Você constrói ou providencia:** o `image.bin`, qualquer arquivo que faça
  o papel de uma imagem de firmware. O `rullst-iot` só decide se uma imagem
  pode ser instalada: ele não baixa, não grava na flash e não reinicia a
  placa, então nesta demonstração nada é instalado de fato. Um caminho real de
  atualização precisa do seu próprio código de download, gravação e
  bootloader.
- **Os alunos aprendem:** hashes (SHA-256), chaves secretas e públicas,
  assinaturas digitais, por que um contador de rollback impede a volta de uma
  versão antiga e vulnerável, e por que a chave secreta nunca sai do
  computador do professor.
- **Dificuldade:** avançada para montar; a demonstração em si serve para
  alunos de nível intermediário.

### 5. Exercícios de programação corrigidos pelo `rullst-labs`

- **Objetivo:** os alunos enviam funções pequenas, por exemplo transformar
  duas leituras de sensores de linha em um valor de direção, e veem quais
  casos de teste passam antes de testar o código em um robô.
- **Placas:** nenhuma para a correção. O código roda em um computador; os
  alunos o copiam para o robô depois.
- **Partes do Rullst:** o `rullst-labs`, como dependência direta (a fachada
  não tem feature para ele). Um `Exercise` tem até 64 `GraderCase`s, cada um
  com dois números inteiros de entrada (`input: [i64; 2]`) e um de saída
  esperado (`expected: i64`), o que serve para direção a partir de duas
  leituras de sensor, velocidade a partir de distância e tempo, ou
  parar-ou-seguir a partir de uma distância e um limite. O `SqliteLabs`
  (feature `sqlite`) guarda as submissões criptografadas, corrige de forma
  exata contra as respostas armazenadas e dá um retorno que nunca revela as
  entradas ocultas nem os valores esperados. O exemplo
  [`course_app`](https://github.com/Rullst/Rullst/blob/main/rullst-labs/examples/course_app.rs)
  mostra o lado da aplicação.
- **O que o host precisa fornecer:** o `rullst-labs` nunca roda o código dos
  alunos; você providencia o seu próprio executor
  ([contrato](../labs-runner-contract.md#verified-by-rullst-labs-versus-guaranteed-by-the-host)).
  Isso é um controlador confiável (features `sqlite` e `receipt-signing`, com
  a própria semente de assinatura Ed25519) que pega os trabalhos e assina os
  resultados, e um worker isolado que compila ou roda cada submissão sem rede,
  sem acesso ao banco de trabalhos nem às chaves, e com limites de CPU,
  memória e tempo
  ([isolamento](../labs-runner-contract.md#isolation-the-host-owns)). A
  aplicação fornece os alunos autenticados e suas matrículas no curso. O
  código-fonte chega como texto (`RustSource`); qual linguagem o executor
  compila ou interpreta é escolha do executor. Na execução de aceitação da
  13.0.0-alpha.5, um executor que iniciava um contêiner Docker por trabalho
  corrigiu submissões em Python, e o contrato afirma que um contêiner sozinho
  não é isolamento suficiente para código hostil.
- **Os alunos aprendem:** escrever uma função a partir de uma especificação,
  casos de teste e casos-limite (zero, leituras negativas), e por que existem
  testes ocultos.
- **Dificuldade:** iniciante para os alunos; avançada para o professor, que
  monta e isola o executor.

## Notas de segurança para as aulas

- **Alimente os motores separadamente.** Acione os motores por um driver de
  motor ou pela placa de driver de um kit de robô, com bateria própria, nunca
  direto de um pino da placa, e ligue o terra da fonte dos motores ao terra da
  placa. Faça os primeiros testes com as rodas fora da mesa.
- **À prova de falhas.** O firmware dos motores para quando os comandos param
  de chegar (por exemplo após meio segundo sem nenhum), e o painel tem um
  botão de parar.
- **Baterias.** Carregue cada bateria com um carregador feito para o tipo
  dela, não deixe baterias de lítio carregando sem supervisão, descarte uma
  bateria estufada, danificada ou quente, e cubra os terminais das baterias
  guardadas.
- **Nenhum segredo no firmware dos alunos.** Os tokens dos dispositivos
  ficam no ambiente do gateway (`IOT_DEVICE_TOKEN` no exemplo), não em sketches
  nem em um repositório; dê a cada placa o seu próprio token e revogue um que
  se perder. Mantenha as placas em uma rede Wi-Fi da sala separada da rede da
  escola, já que a senha dela acaba no firmware. A chave secreta de assinatura
  de firmware fica no computador do professor; as placas guardam só a chave
  pública. O rádio do micro:bit não tem criptografia, então trate o que chega
  por ele como não confiável.

## Para onde ir depois

- [Guia do rullst-iot](../crates/iot.md): o que cada módulo faz, os
  resultados em hardware, assinatura com a CLI e recebimento de telemetria.
- [Tutorial 29: dados de IoT e auxiliares de frames](../tutorials/29-embedded-iot-edge.md).
- [`examples/iot-gateway`](https://github.com/Rullst/Rullst/tree/main/examples/iot-gateway):
  as placas Uno validadas, o gateway, o registro e o painel.
- [Tutorial 34: painel em tempo real com Server-Sent Events](../tutorials/34-realtime-dashboard-sse.md).
- [Contrato do executor do Labs](../labs-runner-contract.md).
- As [ideias para depois da 13.0](../../../ROADMAP.md#ideas-recorded-for-after-130)
  do roadmap, incluindo o exemplo de IoT offline.
- [Issue #637](https://github.com/Rullst/Rullst/issues/637): validar o
  `rullst-iot` em um ESP32-C3. Uma turma com uma placa pode ajudar.

As páginas de referência acima estão em inglês.

## Fontes dos dados das placas

Conferidas em 2026-11-19.

- micro:bit V2: [recursos](https://microbit.org/get-started/features/overview/)
  e [hardware V2.2x](https://tech.microbit.org/hardware/2-2-revision/)
  (micro:bit Foundation); [`microbit-v2`](https://docs.rs/microbit-v2),
  [nrf-rs/microbit](https://github.com/nrf-rs/microbit) e
  [`embassy-nrf`](https://docs.rs/embassy-nrf).
- Arduino Uno R3: [página do produto](https://docs.arduino.cc/hardware/uno-rev3/)
  e [datasheet](https://docs.arduino.cc/resources/datasheets/A000066-datasheet.pdf);
  [Uno R4 WiFi](https://docs.arduino.cc/hardware/uno-r4-wifi/);
  [`avr-hal`](https://github.com/Rahix/avr-hal).
- ESP32-C3 e ESP32-S3: as páginas da Espressif sobre o
  [ESP32-C3](https://www.espressif.com/en/products/socs/esp32-c3) e o
  [ESP32-S3](https://www.espressif.com/en/products/socs/esp32-s3), o
  [datasheet do ESP32-C3](https://documentation.espressif.com/esp32-c3_datasheet_en.pdf),
  a [ESP32-S3-EYE](https://www.espressif.com/en/products/devkits/esp32-s3-eye/overview),
  o [anúncio do esp-hal 1.0](https://developer.espressif.com/blog/2025/10/esp-hal-1/),
  o [capítulo sobre toolchain](https://docs.espressif.com/projects/rust/book/getting-started/toolchain.html)
  do livro de Rust da Espressif, o [`esp-hal`](https://github.com/esp-rs/esp-hal) e
  o [`esp-radio`](https://crates.io/crates/esp-radio).
- Raspberry Pi: [Pico 2](https://www.raspberrypi.com/products/raspberry-pi-pico-2/),
  [documentação da série Pico](https://www.raspberrypi.com/documentation/microcontrollers/pico-series.html),
  [Zero 2 W](https://www.raspberrypi.com/products/raspberry-pi-zero-2-w/),
  [sistemas operacionais](https://www.raspberrypi.com/software/operating-systems/);
  [`embassy-rp`](https://docs.rs/embassy-rp), [`rp235x-hal`](https://docs.rs/rp235x-hal),
  [`cyw43`](https://docs.rs/cyw43) e [`rppal`](https://docs.rs/rppal).
- Alvos do Rust: [suporte a plataformas](https://doc.rust-lang.org/rustc/platform-support.html).
