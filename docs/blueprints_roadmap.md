# Rullst Blueprints Roadmap 🗺️
### *"A Coleção Definitiva de Blueprints de Alta Performance para o Rullst"*

Este documento mapeia o plano de expansão do ecossistema de **Starter Blueprints** do Rullst. O objetivo é fornecer aos desenvolvedores e agências soluções completas e "prontas para produção" que destacam os diferenciais de performance, segurança e produtividade do ecossistema Rust + Rullst.

---

## 🚀 Filosofia de Design dos Blueprints
Cada blueprint adicionado ao CLI deve atender a três princípios fundamentais:
1. **Fator Uau imediato:** Interface linda, responsiva (Dark Mode, Glassmorphism, Micro-animações) e interativa via HTMX/Tailwind.
2. **Uso de Recursos Nativos do Rust/Rullst:** Demonstrar na prática a vantagem injusta do Rust (baixo consumo de RAM, concorrência segura, processamento paralelo, type safety, WebSockets robustos).
3. **Pronto para Produção:** Gerar automaticamente `.env.example`, configurações de banco com travas de concorrência e `.gitignore` blindado.

---

## 🗺️ Roadmap de Novos Blueprints (Ordenado do Mais Fácil ao Mais Difícil)

| ID | Nome do Blueprint | Foco Técnico no Rullst | Diferencial Comercial |
|:---|:---|:---|:---|
| **4** | 💼 ERP Pocket (Estoque) | SQLite Embarcado + `rullst::nexus` (Auto-CMS) + Binário Único | Pequenas e médias empresas com sistema offline-first imune a quedas. |
| **5** | 📡 Uptime Monitoring Service | Asynchronous Workers (`rullst::queue`) + Health Checks | Alternativa ao Uptime Kuma rodando em VPS de $5 com zero uso de memória. |
| **6** | 📋 Gestão de Membros / Clubes | `#[derive(Validate)]` + Nexus + Geração de PDF de Recibos | Cadastro de pessoas pagantes para academias, clubes e condomínios. |
| **7** | 🤖 AI Agent & RAG Boilerplate | `rullst::ai` (Ollama/Gemini/OpenAI) + Vector Embedding local | Análise inteligente de documentos PDF/TXT locais e privados. |
| **8** | 🪙 AI Credit-Based SaaS | Streaming (SSE) + `rullst-orm` (Lock Concorrência) + Stripe | Plataformas SaaS de IA com consumo de tokens seguro contra race conditions. |
| **9** | 🏥 Agendamentos & Clínicas | Calendário HTMX + Cron Scheduler + Locks de Double-Booking | Barbearias, médicos e profissionais autônomos com prevenção de reservas duplicadas. |
| **10**| 🚪 Controle de Acesso Biométrico | `rullst::routing` (WebSockets) + Painel Portaria Real-time | Sistemas de portaria, academias e ponto eletrônico sem lag. |
| **11**| 📈 Checkout de Afiliados | SSR Rápido (<100ms) + Split de Comissões + Landing Page | Páginas de vendas com Score 100 no Lighthouse para conversão máxima. |
| **12**| 🏢 B2B Multi-Tenant Platform | `rullst::multitenant` (subdomínios) + RBAC (Enums) + `rullst::mail` | Software corporativo isolado e seguro para venda de licenças corporativas. |
| **13**| 💬 Discord-Like Realtime Chat | **Rullst Live** (Server-Driven UI) + WebSockets no Tokio | Salas de chat concorrentes escaláveis a baixo custo de infraestrutura. |
| **14**| 🛵 Delivery / iFood | Fila de Background (`rullst::queue`) + Estado de Pedido | Processamento assíncrono de status de entrega e envio de e-mails. |

---

## 🔍 Detalhamento Arquitetural de Destaque

### 🪙 8. AI Credit-Based SaaS (The Token-Burner)
* **Arquitetura:** Interface de chat limpa consumindo dados por Server-Sent Events (SSE) nativo para streaming fluído de respostas de IA.
* **Segurança de Dados:** O `rullst-orm` implementa locks estritos de transação para garantir que, caso o saldo de créditos do usuário seja zerado simultaneamente em duas abas distintas, o sistema aborte a geração de tokens antes de finalizar chamadas custosas ao modelo LLM.
* **Monetização:** Checkout Stripe integrado com faturamento baseado em uso e portal de cobrança auto-gerenciado.

### 🏢 12. B2B Multi-Tenant Platform (The Corporate Boilerplate)
* **Isolamento:** Uso do módulo nativo `rullst::multitenant` que intercepta requisições HTTP e injeta dinamicamente o escopo do `tenant_id` em todas as consultas SQL do ciclo de vida da requisição, evitando vazamento acidental de dados entre empresas.
* **Permissões (RBAC):** Estrutura de cargos (`Admin`, `Member`, `Billing`) baseada em enums seguros em Rust, validada via middlewares antes de despachar para os controllers.
* **Convites:** Fluxo de convite via e-mail tokenizado criptograficamente com expiração de 24 horas usando o mailer nativo `rullst::mail`.

### 💬 13. Discord-Like Realtime Chat
* **Sem JS Complexo:** Uso do **Rullst Live** para manter o estado da sala de bate-papo no servidor. Toda mensagem submetida via formulário HTMX é processada, adicionada ao canal de broadcasting da thread e renderizada diretamente pelo servidor, atualizando o DOM dos clientes via WebSockets em tempo real.
* **Escala:** Utilização das threads eficientes do runtime `Tokio` do Rust, permitindo milhares de conexões WebSocket persistentes ativas consumindo menos de 50MB de RAM no servidor.

### 🏥 9. Agendamentos & Clínicas (The Scheduler)
* **Prevenção de Conflitos:** Transações do banco executadas com nível de isolamento estrito `SERIALIZABLE` ou lock pessimista para impedir double-booking de horários no milissegundo de confirmação.
* **Cron Integrado:** Registro de lembretes via Cron nativo do Rullst para buscar agendamentos das próximas 2 horas e disparar notificações automáticas sem necessidade de agendadores externos como Sidekiq ou Celery.

### 🤖 7. AI Agent & RAG Boilerplate (AI-Native)
* **Estrutura:** Interface intuitiva para upload de arquivos onde o Rullst converte o documento, calcula as embeddings utilizando modelos locais ou APIs configuradas e armazena os dados vetoriais no banco de dados SQLite embarcado.
* **Flexibilidade:** Configuração flexível via `rullst::ai` permitindo alternar instantaneamente entre LLMs comerciais externas (Gemini, OpenAI) e instâncias locais (Ollama / Llama 3) com uma única variável de ambiente.
