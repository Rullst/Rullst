//! The plain words of the benchmark blocks, by page language. Sentences with markup or
//! values are written with `html!` where they are used.

use crate::pages::Lang;

pub struct Words {
    pub run: &'static str,
    /// The comparison table's column titles, Overall second.
    pub headers: [&'static str; 8],
    pub minimal_name: &'static str,
    pub missing: &'static str,
    pub unpublished: &'static str,
    pub legend: &'static str,
    pub memory: &'static str,
    pub reading: &'static str,
    /// The flags' explanations, in the order of `FLAGS`.
    pub flags: [&'static str; 4],
}

const EN: Words = Words {
    run: "measurement run",
    headers: [
        "Framework",
        "Overall",
        "Hello world",
        "Database",
        "Application",
        "Memory",
        "Startup",
        "Stability",
    ],
    minimal_name: "Rullst (minimal)",
    missing: "Not in this measurement run.",
    unpublished: "This run publishes no overall score; the categories are the result.",
    legend: "Each cell shows the position (a range such as 3–7 is a tie: those implementations share \
             positions 3 to 7 in no particular order), the score and its tie margin (±: the margin of its \
             comparison with the score above; the first position has none).",
    memory: "Memory",
    reading: "What this means for Rullst",
    flags: [
        "* In at least one run of this score the runner's load generator and PostgreSQL were the limit \
         (client-bound), so that run counts with its CPU-cost ratio, requests per CPU-second of the \
         server and PostgreSQL together, instead of requests per second.",
        "† This implementation's database setup differs from the others' in a way the full results explain.",
        "‡ In this run most of this implementation's measured PUT /world/{id} requests did not write a \
         row, so its Application score, and through it its overall score, are higher than they would be \
         with a write. The full results explain the cause and its fix.",
        "≈ Within the tie margin of Axum's 1.00×: not distinguishable from Axum.",
    ],
};

const PT: Words = Words {
    run: "execução da medição",
    headers: [
        "Framework",
        "Geral",
        "Hello world",
        "Banco de dados",
        "Aplicação",
        "Memória",
        "Inicialização",
        "Estabilidade",
    ],
    minimal_name: "Rullst (mínimo)",
    missing: "Fora desta rodada de medição.",
    unpublished: "Esta rodada não publica pontuação geral; o resultado são as categorias.",
    legend: "Cada célula mostra a posição (um intervalo como 3–7 é um empate: essas implementações \
             dividem as posições 3 a 7, sem ordem entre elas), a pontuação e a margem de empate dela (±: a \
             margem da comparação com a pontuação acima; a primeira posição não tem).",
    memory: "Memória",
    reading: "O que isto significa para o Rullst",
    flags: [
        "* Em pelo menos uma medição desta pontuação o gerador de carga e o PostgreSQL da máquina eram o \
         limite (client-bound), então essa medição conta com a razão de custo de CPU, requisições por \
         segundo de CPU do servidor e do PostgreSQL juntos, em vez de requisições por segundo.",
        "† A configuração de banco de dados desta implementação difere da das outras de um jeito que os \
         resultados completos explicam.",
        "‡ Nesta rodada, a maioria das requisições PUT /world/{id} medidas desta implementação não gravou \
         nenhuma linha, então a pontuação de Aplicação dela, e por meio dela a pontuação geral, são mais \
         altas do que seriam com uma gravação. Os resultados completos explicam a causa e a correção.",
        "≈ Dentro da margem de empate do 1,00× do Axum: não dá para distinguir do Axum.",
    ],
};

pub fn words(lang: Lang) -> &'static Words {
    if lang.is_pt() { &PT } else { &EN }
}

impl Words {
    /// A category's column title on the comparison page.
    pub fn category(&self, key: &str) -> &'static str {
        super::CATEGORIES
            .iter()
            .position(|category| *category == key)
            .and_then(|index| self.headers.get(index + 2))
            .copied()
            .unwrap_or("")
    }

    /// A flag's explanation.
    pub fn flag(&self, flag: &str) -> &'static str {
        super::FLAGS
            .iter()
            .position(|(key, _)| *key == flag)
            .and_then(|index| self.flags.get(index))
            .copied()
            .unwrap_or("")
    }

    /// A row's name: the comparison's names, with the minimal row translated.
    pub fn row_name(&self, slug: &str) -> &'static str {
        if slug == super::MINIMAL {
            self.minimal_name
        } else {
            super::row_name(slug)
        }
    }
}

/// A status as the pages write it ("ranked" has no translation: a ranked cell has a score).
pub fn status(status: &str, lang: Lang) -> &str {
    let pairs: [(&str, &str, &str); 6] = [
        ("diagnostic", "diagnostic", "diagnóstico"),
        ("errors", "errors", "erros"),
        ("incomplete", "incomplete", "incompleto"),
        ("not comparable", "not comparable", "não comparável"),
        ("not measured", "not measured", "não medido"),
        ("unstable", "unstable", "instável"),
    ];
    pairs
        .iter()
        .find(|(key, _, _)| *key == status)
        .map_or(status, |(_, en, pt)| if lang.is_pt() { pt } else { en })
}

/// A stability result as the pages write it.
pub fn stability(result: &str, lang: Lang) -> &'static str {
    match (result, lang.is_pt()) {
        ("pass", false) => "pass",
        ("warn", false) => "warn",
        ("fail", false) => "fail",
        ("pass", true) => "passou",
        ("warn", true) => "alerta",
        ("fail", true) => "falhou",
        _ => "–",
    }
}
