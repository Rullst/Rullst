# Hardening status — rastreabilidade da avaliação `gpt.md`

> Estado pontual do worktree em **2026-08-28**, sobre o `HEAD` base
> `d2e5580a` e as alterações locais auditadas a seguir. Este documento
> mapeia as recomendações da [avaliação técnica](../../gpt.md); não é certificado,
> pentest, homologação de provedor nem declaração geral de production-readiness.

## Como ler este relatório

As classificações abaixo têm significado estrito:

- **Corrigido**: o defeito concreto descrito em `gpt.md` tem implementação e
  regressão local correspondente. Isso não amplia o escopo da capacidade.
- **Mitigado / fail-closed**: o comportamento inseguro ou enganoso não retorna
  sucesso, mas a integração real continua indisponível ou depende de uma
  infraestrutura externa ainda não implementada.
- **Parcial**: parte material do item foi entregue, mas o critério completo ainda
  não é demonstrável.
- **Pendente**: não foi encontrada mitigação suficiente no estado inspecionado.

A [SST](spec.md) continua sendo o contrato normativo. O
[capability ledger](capability-ledger.md) registra o que é implementado,
experimental, deliberadamente não prometido ou visão futura; o
[ROADMAP canônico](../../ROADMAP.md), incluído na [versão do livro](roadmap.md),
prioriza o trabalho restante. Este relatório não substitui nem promove itens do
ledger/ROADMAP: ele apenas liga cada achado histórico à evidência atual.

## Resumo executivo

| Grupo | Corrigido | Mitigado / fail-closed | Parcial | Pendente |
|---|---:|---:|---:|---:|
| P0-01..09 | 6 | 3 | 0 | 0 |
| P1-01..19 | 16 | 1 | 2 | 0 |
| P2-01..21 | 17 | 2 | 2 | 0 |
| **Total dos 49 achados** | **39** | **6** | **4** | **0** |

“Sem pendência” nessa tabela significa que cada achado possui ao menos uma
correção ou contenção; não significa que as capacidades fail-closed foram
implementadas. NFS-e real, Alipay RSA2, storage remoto, replicação genérica,
transports MQTT/CoAP, HSM/PQC e os testes reais multi-instância/failover do rate limiting
distribuído continuam fora do contrato entregue, como registra o capability
ledger. O adapter Redis opcional é uma fundação implementada, não evidência de
uma topologia de produção validada.

## Achados críticos — P0

| ID | Estado | Evidência atual e limite |
|---|---|---|
| **P0-01 — NFS-e/XMLDSig inválida** | **Mitigado localmente / live fail-closed** | [`fiscal/signer.rs`](../../rullst-capital/src/fiscal/signer.rs) agora rejeita envelopes/IDs/certificados inválidos e produz XMLDSig envelopada RSA-SHA256/C14N inclusiva 1.0 a partir de PKCS#12; a assinatura é verificada localmente e o XML assinado passa o XSD oficial checksum-pinned quando o pacote é fornecido. [`fiscal/protocol.rs`](../../rullst-capital/src/fiscal/protocol.rs) gera o envelope GZip/Base64 determinístico e só classifica HTTP 201 como autorização após vincular ambiente, DPS, chave, `infNFSe` e XMLDSig; rejeições e input malformado/tampered/bomb permanecem distintos. [`fiscal/schema.rs`](../../rullst-capital/src/fiscal/schema.rs) limita arquivos, hashes, resolução e tamanho; [`fiscal/client.rs`](../../rullst-capital/src/fiscal/client.rs) constrói mTLS rustls limitado, mas não transmite. Regressões positivas/negativas cobrem signer, protocolo, XSD, credencial mock e contratos de ambiente. Política ICP-Brasil/emissor, idempotência, A1 real na produção restrita, revisão independente e homologação SEFIN continuam abertos; os modos reais falham fechados. |
| **P0-02 — `FieldEncryptor` irreversível/falso** | **Corrigido** | [`vault.rs`](../../rullst-security/src/vault.rs) usa envelope versionado AES-256-GCM, nonce aleatório, AAD e key-id/keyring, rejeitando chave de tamanho incorreto e envelope legado ambíguo. Round-trip, adulteração e chave incorreta são cobertos em [`mfa_sri_vault_test.rs`](../../rullst-security/tests/mfa_sri_vault_test.rs); [`vault/tests.rs`](../../rullst-security/src/vault/tests.rs) também prova rejeição de nonce/ciphertext malformado, campos extras, key-id excessivo e plaintext autenticado não UTF-8. |
| **P0-03 — stubs criptográficos IoT expostos como garantias** | **Mitigado / fail-closed** | [`ota.rs`](../../rullst-iot/src/ota.rs) verifica assinatura Ed25519 de manifesto canônico, hash/tamanho/target e anti-rollback antes de permitir commit; APIs legadas falham com erro tipado. Os antigos bytes HSM/PQC e o formatador de valor MQTT foram renomeados para `Simulated*` e isolados por `experimental-simulators`. Os novos [`mqtt.rs`](../../rullst-iot/src/mqtt.rs) e [`coap.rs`](../../rullst-iot/src/coap.rs) são somente encoders de pacotes bounded, sem rede ou criptografia. Vetores e negativas ficam nos testes IoT. Transporte, hardware e ML-KEM reais continuam roadmap. |
| **P0-04 — Nexus destrutivo aberto por padrão** | **Corrigido** | [`nexus/mod.rs`](../../rullst-nexus/src/nexus/mod.rs) exige política de autenticação em `try_build`, devolve `MissingAuthenticationPolicy` sem ela e instala `RequireRoleLayer<NexusPrincipal>`; construtores legados são deny-all/deprecados. [`access.rs`](../../rullst-nexus/src/nexus/access.rs) exige credenciais fortes, peer rate limit e fronteira TLS verificável, e `NexusAuthPolicy::protect_router` aplica a mesma fronteira a rotas administrativas da aplicação. Testes: [`access/tests.rs`](../../rullst-nexus/src/nexus/access/tests.rs) e casos de build fail-closed no próprio módulo. |
| **P0-05 — CORS reflect-origin gerado** | **Corrigido** | [`cors_middleware.rs.template`](../../cargo-rullst/src/generators/cors_middleware.rs.template) gera allowlist tipada, rejeita wildcard/origens inválidas, não habilita credenciais por padrão e limita métodos. Regressões do gerador estão em [`cors_jwt.rs`](../../cargo-rullst/src/generators/cors_jwt.rs). A etapa separada de aviso a consumidores antigos é tratada na Fase 0 abaixo. |
| **P0-06 — storage confirma persistência inexistente/path traversal** | **Corrigido / upload parcial** | [`storage.rs`](../../rullst-core/src/storage.rs) valida componentes, confina o caminho ao diretório canônico, barra escape por symlink e retorna `Unsupported` para S3/R2/resize. [`TenantStorage`](../../rullst-core/src/storage/tenant.rs) liga o namespace local a `TenantContext` autenticado e prova não-interferência da mesma chave. [`uploads.rs`](../../rullst-core/src/uploads.rs) acrescenta admissão/quarentena storage-agnostic com limite/tipo/nome/tenant, assinatura versus MIME/extensão, digest e scanner fail-closed; multipart, S3/R2, parsing profundo e scanner real continuam abertos. Testes incluem os negativos de persistência/path, `TM-TENANT-04` e `TM-ACADEMY-09`. |
| **P0-07 — ambiente de produção fail-open/inconsistente** | **Corrigido** | [`config.rs`](../../rullst-core/src/config.rs) define `Environment` e precedência validada `RULLST_ENV` → `APP_ENV` → config; valores inválidos são erros. [`server/builder.rs`](../../rullst-core/src/server/builder.rs) carrega dotenv sem mutar o ambiente global, propaga falhas de config/DB e instala [`apply_security_baseline`](../../rullst-core/src/security/baseline.rs) em staging/produção. Essa composição injeta a configuração da aplicação antes de headers/CSP nonce, CORS exato, WAF, CSRF e PII opcional e possui regressão HTTP integrada; browser/proxy/TLS reais permanecem gate de deploy. Regressões: `environment_resolution_has_one_precedence_and_validated_aliases` e `production_baseline_composes_nonce_cors_csrf_and_headers`. |
| **P0-08 — integridade de providers/webhooks** | **Mitigado / fail-closed** | [`providers/mod.rs`](../../rullst-capital/src/providers/mod.rs) rejeita segredo real vazio e separa mock explícito; verificadores com timestamp aplicam freshness. [`webhook.rs`](../../rullst-capital/src/webhook.rs) adiciona replay store limitado e modo produção que rejeita mock. [`alipay.rs`](../../rullst-capital/src/providers/alipay.rs) retorna `UnsupportedOperation` no modo RSA2 real, em vez de simular HMAC como RSA. Regressões incluem `explicit_mock_webhook_mode_still_requires_its_secret`, `replay_store_rejects_duplicates_and_expires_entries` e `replay_store_is_bounded_and_provider_scoped`. O replay store ainda é em memória e não resolve idempotência entre processos. |
| **P0-09 — release fora da ordem topológica** | **Corrigido** | [`.github/workflows/release.yml`](../../.github/workflows/release.yml) verifica fmt/Clippy/testes, empacota todos os crates antes do primeiro publish e publica pela DAG real, incluindo Connect/IoT e ORM antes de Core; valida tag/versões, checksums e artefatos. É evidência estrutural do workflow, não evidência de que uma execução de release passou. |

## Achados altos — P1

| ID | Estado | Evidência atual e limite |
|---|---|---|
| **P1-01 — invariantes WebAuthn incompletas** | **Parcial** | [`passkey/service.rs`](../../rullst-auth/src/auth/passkey/service.rs) e [`ceremony.rs`](../../rullst-auth/src/auth/passkey/ceremony.rs) validam tipo da cerimônia, origin/rpIdHash, cross-origin, UP/UV, COSE ES256, coordenadas X/Y, raw-id, contador monotônico e challenges compartilhados, bounded, one-time e com TTL. Os negativos, inclusive coordenadas ausentes, estão em [`invariant_tests.rs`](../../rullst-auth/src/auth/passkey/invariant_tests.rs). O escopo de attestation permanece deliberadamente estreito (`none`/ES256) e ainda não há prova por suíte normativa ou biblioteca WebAuthn auditada; o ledger mantém “Full normative WebAuthn server” como parcial. |
| **P1-02 — sessão/APP_KEY/rehash/legado** | **Corrigido** | [`auth.rs`](../../rullst-auth/src/auth.rs) valida força/placeholder do segredo nos ambientes seguros, compara algoritmo/versão/parâmetros Argon2 para rehash e só aceita envelope de sessão versionado e expirável. [`app_key_resolution.rs`](../../rullst-auth/tests/app_key_resolution.rs) isola processos para provar precedência, config inválida, ambiente não Unicode, persistência privada da chave dev e cookie `Secure` em produção; outras regressões cobrem sessão sem versão, `needs_rehash` e expiração. O JWT de aplicação continua explicitamente responsabilidade do scaffold, conforme o ledger. |
| **P1-03 — DLP/PII corrompem respostas** | **Corrigido** | [`dlp.rs`](../../rullst-security/src/dlp.rs) e [`pii.rs`](../../rullst-core/src/security/pii.rs) só reescrevem texto bufferizável suportado; binário, streaming, encoding e overflow têm bypass/erro explícito sem UTF-8 lossy ou falso sucesso, e headers de representação são recalculados/removidos. Regressões: `invalid_utf8_and_incomplete_pem_are_not_corrupted`, `test_dlp_layer_middleware` e [`core/security/tests.rs`](../../rullst-core/src/security/tests.rs). |
| **P1-04 — CSRF incompatível com webhooks** | **Mitigado / fail-closed** | [`csrf.rs`](../../rullst-core/src/security/csrf.rs) reconhece métodos seguros e apenas isenções POST exatas configuradas; o scaffold monta a rota de webhook assinada separadamente em [`saas/routes.rs`](../../cargo-rullst/src/blueprints/saas/routes.rs). Capital exige assinatura/freshness/replay. Testes: `safe_http_methods_do_not_require_a_token` e `only_exact_configured_post_webhook_path_is_exempt`. A idempotência cross-instance ainda requer backend compartilhado. |
| **P1-05 — WAF/RASP não inspecionam body** | **Corrigido** | [`core/security/waf.rs`](../../rullst-core/src/security/waf.rs) e [`security/rasp.rs`](../../rullst-security/src/rasp.rs) inspecionam bodies text/JSON/form limitados, decodificam strings JSON, falham fechados em tamanho/encoding inválido e reconstroem a request. Regressões: `waf_inspects_and_preserves_bounded_request_bodies`, `json_body_inspection_decodes_escaped_strings` e `middleware_fails_closed_for_uninspectable_textual_bodies` em [`rasp/tests.rs`](../../rullst-security/src/rasp/tests.rs). Continuam heurísticas de defesa em profundidade, não substitutos para parser, bind ou autorização. |
| **P1-06 — CSP não corresponde a nonces/A+** | **Corrigido** | [`core/security/headers.rs`](../../rullst-core/src/security/headers.rs) e [`security/headers.rs`](../../rullst-security/src/headers.rs) emitem baseline sem `unsafe-inline`/`unsafe-eval` e expõem `CspNonce` ao renderer; policy estática continua presente quando nonce dinâmico está desligado. Testes verificam que nonce do header e extension coincidem em [`core/security/tests.rs`](../../rullst-core/src/security/tests.rs) e [`security_tests.rs`](../../rullst-security/tests/security_tests.rs). A documentação não deve prometer nota universal de scanner. |
| **P1-07 — middleware de webhook perde request parts/body** | **Corrigido** | [`webhook.rs`](../../rullst-capital/src/webhook.rs) usa `into_parts`/`from_parts`, mantém método, URI, versão, headers, extensions e bytes originais, além do evento verificado. Regressão: `reconstructed_request_preserves_parts_extensions_and_body`. |
| **P1-08 — inicialização ORM global parcial/panicking** | **Corrigido** | [`pool.rs`](../../rullst-orm/src/pool.rs) prepara primária/réplicas localmente com `try_join_all` e só então publica um único `OnceLock<OrmState>`; getters normais retornam `Result`. [`atomic_init_test.rs`](../../rullst-orm/tests/atomic_init_test.rs) prova que falha de réplica não publica estado parcial. |
| **P1-09 — janitor limpa clones do Login Guard** | **Corrigido** | [`login_guard.rs`](../../rullst-security/src/login_guard.rs) limpa diretamente os `DashMap` ativos via `cleanup_if_due`, aplica TTL e `max_identities`, sem task dona de clones divergentes. Testes `test_login_guard_tarpit_and_jail`, `test_login_guard_global_and_expired_jail` e concorrência em [`concurrency_tests.rs`](../../rullst-security/tests/concurrency_tests.rs). |
| **P1-10 — distributed rate limit no-op/IP forjável** | **Corrigido** | [`rate_limit/redis.rs`](../../rullst-security/src/rate_limit/redis.rs), sob `redis-rate-limit`, usa script Lua atômico, namespace validado, TTL e chave de cliente SHA-256; empty/`mock_*` é modo local explícito e `require_distributed()` falha fechado. O selector legado sem configuração continua `DistributedBackendUnsupported`. [`resilience.rs`](../../rullst-core/src/resilience.rs) ignora forwarded headers no extractor padrão e permite política explícita de proxy confiável. Além das regressões offline, [`redis_rate_limit_live.rs`](../../rullst-security/tests/redis_rate_limit_live.rs) prova que duas instâncias independentes compartilham o mesmo budget num Redis real; CI e release sobem uma imagem fixada por digest. Failover/cluster e composição HTTP continuam gates separados. |
| **P1-11 — XSS/autorização visual no Nexus** | **Corrigido** | [`ai_chat.rs`](../../rullst-nexus/src/nexus/ai_chat.rs) sanitiza saída externa; [`crud/views.rs`](../../rullst-nexus/src/nexus/crud/views.rs) escapa/encodeia IDs sem JS inline; [`crud/handlers.rs`](../../rullst-nexus/src/nexus/crud/handlers.rs) aplica `hidden`/`readonly`, limita batch e propaga erro; paginação é limitada/saturating. O router inteiro exige papel admin em [`nexus/mod.rs`](../../rullst-nexus/src/nexus/mod.rs). Testes de acesso e CRUD estão em [`rullst-nexus/tests`](../../rullst-nexus/tests). |
| **P1-12 — Studio inventa métricas/rota/XSS/env leak** | **Corrigido** | [`radar.rs`](../../rullst-core/src/radar.rs) representa probes ausentes como `Option` e mede CPU real por deltas em Linux e Windows; [`radar_visualizer.rs`](../../rullst-studio/src/radar_visualizer.rs) atualiza os KPIs via `/api/radar` sem converter ausência em sucesso; [`security_radar.rs`](../../rullst-studio/src/security_radar.rs) usa `Unavailable`, rota coerente e DOM `textContent`/`replaceChildren`; [`env_viewer.rs`](../../rullst-studio/src/env_viewer.rs) usa política de redaction; [`feature_flags.rs`](../../rullst-studio/src/feature_flags.rs) encodeia nomes e usa binds corretos. A construção exige [`LocalStudioAccess`](../../rullst-studio/src/access.rs), que só existe como opção efetiva em debug e nega peer remoto ou `ConnectInfo` ausente. Regressões cobrem as páginas, a sonda Windows e a fronteira HTTP (`200/403/403`). Um Studio compartilhado com autenticação embutida continua explicitamente não implementado. |
| **P1-13 — regressões concretas dos geradores** | **Parcial** | Flags, IDs estáveis, path/package, Island/Resource, Auth, Billing e Docs SSG têm correções/testes em [`cargo-rullst/src`](../../cargo-rullst/src). [`scaffold_contracts.rs`](../../cargo-rullst/tests/scaffold_contracts.rs) valida paths, Rust e `Cargo.toml` em **270 combinações estruturais**, materializa os seis blueprints e aplica o scanner IDOR; [`generated_saas_check.rs`](../../cargo-rullst/tests/generated_saas_check.rs) executa checks Cargo reais sobre sete casos que cobrem os seis blueprints, três padrões ORM, cinco frontends, API, banco, hot reload e um build release. [`generated_lms_modules_check.rs`](../../cargo-rullst/tests/generated_lms_modules_check.rs) acrescenta três perfis destacados, inclusive assessment sem gamificação/outbox, totalizando dez casos materializados. O caso LMS aplica suas migrations em SQLite e prova as fronteiras HTTP autenticadas de autoria, publicação, rollback editorial, tarefa/rubrica/submissão/feedback/correção, conclusão/certificado e do ciclo grant/revoke; a regressão cobre revisão/pin imutável, rollback como nova revisão com autorização/replay/conflito/auditoria e preservação de pins antigos/novos, tarefa owner-only com prazo/tentativas, avaliação por papel limitada aos critérios persistidos, correção administrativa append-only com before/after/effective grade e negativos de replay/conflito/pontuação impossível, conclusão por ruleset fixado com incompletude/cross-user/replay/verificação sem PII/revogação auditada, papéis educacionais duráveis com expiração/revogação e grants privilegiados separados, política fail-closed de liberação/expiração/pré-requisito, progresso monotônico/auditado, quiz autoritativo com tempo/ordem persistidos e projeção atômica em `ScoreEvent`/leaderboard/outbox, exercícios owner-only single-choice, matching e typed com tentativa durável, binding transacional da configuração, pares/texto delimitados e replay exato/conflitante, além de agenda `rullst-box-v1` durável no score com replay sem avanço e fila owner/school/enrollment-scoped, worker supervisionado, scheduler de publicação supervisionado com contenção/ativação/replay, APIs owner-only de notificação, correções, backoff, recuperação de lease e rejeição de token obsoleto. O gate [`.github/test-packaged-distribution.sh`](../../.github/test-packaged-distribution.sh) instala o CLI extraído do `.crate`, gera os seis blueprints sem paths do monorepo e os compila offline. Ainda faltam a repetição no SHA da RC, `fmt/smoke` mais amplo e ambientes de contrato para comandos externos. |
| **P1-14 — guardrails/mocks/DeepSeek/structured output** | **Corrigido** | [`guardrails.rs`](../../rullst-ai/src/ai/guardrails.rs) integra o estágio ao client de alto nível e providers repetem a proteção em chamadas diretas; [`deepseek.rs`](../../rullst-ai/src/ai/providers/deepseek.rs) existe; empty/`mock_*` é offline determinístico em chat/vision/embeddings. [`structured.rs`](../../rullst-ai/src/ai/structured.rs) diferencia JSON parseável de schema nativo e retorna `UnsupportedCapability` quando o provider não o garante. [`guardrails_pipeline_test.rs`](../../rullst-ai/tests/guardrails_pipeline_test.rs) percorre providers/capacidades e prova determinismo sem endpoint. Schema nativo em todos os LLMs continua visão, não claim. |
| **P1-15 — constructors/OIDC/JWKS de Connect** | **Corrigido** | Constructors gerados são fallible e aceitam `impl Into<String>` em [`macros.rs`](../../rullst-connect/src/macros.rs); credenciais mock/empty não podem ser redirecionadas a endpoint real. [`oidc/discovery.rs`](../../rullst-connect/src/providers/oidc/discovery.rs) valida URL/host/issuer/endpoints, e o client HTTP desabilita redirects. [`provider/jwks.rs`](../../rullst-connect/src/provider/jwks.rs) implementa TTL, refresh por `kid` desconhecido e stale limitado. Negativos/rotação estão em [`providers/oidc/tests.rs`](../../rullst-connect/src/providers/oidc/tests.rs) e [`provider/tests.rs`](../../rullst-connect/src/provider/tests.rs). |
| **P1-16 — invariantes de Mail/tenant/tracking** | **Corrigido** | [`pipeline.rs`](../../rullst-mail/src/pipeline.rs), [`facade.rs`](../../rullst-mail/src/facade.rs) e [`worker.rs`](../../rullst-mail/src/worker.rs) centralizam CRLF, deliverability, links e tenant; o facade persiste scheduling delimitado pela Queue e o worker recusa claim antecipado antes de consumir o timestamp. [`resolver.rs`](../../rullst-mail/src/resolver.rs) liga diretamente o `TenantContext` autenticado do Core ao driver in-process, valida o registro e falha fechado se o lock estiver indisponível, com regressão de não interferência entre dois contextos; [`error.rs`](../../rullst-mail/src/error.rs) tipa falha permanente/transiente/rate-limit, limita/redige respostas HTTP e [`failover.rs`](../../rullst-mail/src/drivers/failover.rs) suprime fallback para configuração, validação, HTTP 4xx não-429 e SMTP permanente, mantendo decisão estruturada sem body no tracing; action URLs exigem HTTP(S), host e ausência de credenciais; [`drivers/mock.rs`](../../rullst-mail/src/drivers/mock.rs) seleciona offline deterministicamente; [`tracking.rs`](../../rullst-mail/src/tracking.rs) exige segredo forte, HMAC constant-time, TTL e replay store limitado. Os sete scaffolds, inclusive fiscal com proveniência explícita e dunning D+1/D+3/D+7, passam o contrato materializado em [`mail_scaffold_cli.rs`](../../cargo-rullst/tests/mail_scaffold_cli.rs); demais regressões estão nos testes de tracking, pipeline, scheduling de queue e integração Mail. |
| **P1-17 — panics nos caminhos apontados** | **Corrigido** | Os caminhos citados foram convertidos a erros/fallbacks: client Wasm e server em [`rullst-core/src`](../../rullst-core/src), cache global e getters ORM, expansão de macros e Auth/Island/Omni gerados. [`.github/workflows/zero-panics.yml`](../../.github/workflows/zero-panics.yml) cobre bibliotecas, `rullst-macros`, CLI/binários, Wasm e testes de expansão; a auditoria AST genérica rejeita `unwrap`/`expect`/`panic!`/`todo!`/`unimplemented!` em literais de código runtime dos geradores. O escopo não inclui dependências, OOM ou falhas do host. |
| **P1-18 — tenant escolhido pelo cliente** | **Corrigido no boundary definido** | [`tenant_guard.rs`](../../rullst-core/src/security/tenant_guard.rs) deriva seleção de `TenantMembership`/`TenantContext` confiável e ignora headers como autoridade; [`multitenant.rs`](../../rullst-core/src/multitenant.rs) trata header/query/subdomínio apenas como seletor sujeito a membership, devolvendo 403 sem vínculo. [`RbacGuard`](../../rullst-security/src/rbac/guard.rs) carrega tenant validado e exige match exato sem bypass de admin; [`TenantStorage`](../../rullst-core/src/storage/tenant.rs), [`TenantCache`](../../rullst-core/src/cache/tenant.rs), [`TenantRealtime` e `TenantPresence`](../../rullst-core/src/realtime.rs) oferecem namespaces locais ligados ao mesmo contexto. O LMS gerado persiste escolas/memberships/coortes/entitlements, resolve a escola no middleware e filtra mutações/leaderboard; outbox, automação derivada e notificações preservam `school_id`, o leaderboard integra cache local tenant-scoped e notificações novas podem ser projetadas para uma assinatura realtime tenant/user autenticada. Os testes negam seleção arbitrária/ambígua, regra estrangeira, vazamento de notificação do mesmo usuário, admin cross-school e colisões locais da mesma chave/canal/sala de presença. Isso não cobre anexos/mídia, demais caches, autorização ampla de salas, transporte distribuído, storage remoto, busca, métricas, exports, Nexus, cache distribuído ou bancos reais. |
| **P1-19 — bypass CSWSH por prefixo de host** | **Corrigido** | [`cswsh.rs`](../../rullst-security/src/cswsh.rs) normaliza e compara esquema/host/porta exatos, com origin ausente fechado por padrão. Testes `deceptive_localhost_prefixes_are_rejected` e `middleware_rejects_a_deceptive_localhost_origin` cobrem `localhost.evil`. |

O teto local posterior de Mail complementa P1-16 com três wrappers opt-in:
[`inspection.rs`](../../rullst-mail/src/inspection.rs) falha antes do transporte
em assinaturas conhecidas incompatíveis, conteúdo ativo e indisponibilidade do
scanner; [`suppression`](../../rullst-mail/src/suppression/mod.rs) oferece store
bounded process-local ou SQLite compartilhado-local com replay e quotas
transacionais; e [`observability.rs`](../../rullst-mail/src/observability.rs)
omite destinatário, assunto, corpo e filenames. As provas exatas entram em
`TM-MAIL-01` a `TM-MAIL-03`. Isso não equivale a antivírus/CDR, autenticação de
webhook do provider, replicação multi-host, operação de telemetria ou inbox
delivery.

## Achados médios — P2

| ID | Estado | Evidência atual e limite |
|---|---|---|
| **P2-01 — audit chain ambígua/sem continuidade** | **Corrigido** | [`audit/chain.rs`](../../rullst-security/src/audit/chain.rs) usa serialização domain-separated e length-prefixed, chave forte, sequência atômica e commit somente após o logger; valida continuidade. Testes: `length_prefixing_prevents_delimiter_collisions`, `weak_keys_are_rejected` e `logger_failure_does_not_create_a_sequence_gap`. Persistência durável continua responsabilidade de um sink. |
| **P2-02 — telemetria finge HMAC/IP/tempo** | **Corrigido** | [`telemetry.rs`](../../rullst-security/src/telemetry.rs) força `verified_hmac=false` para evento local, não inventa loopback para IP inválido, emite RFC3339 absoluto e não duplica o total de prompts inspecionados ao registrar um bloqueio. O [`DurableSiemSpool`](../../rullst-security/src/siem/spool.rs) mantém essa proveniência falsa desabilitada ao persistir, delimita bytes/registros, chama `sync_data` e valida versão/comprimento/SHA-256/JSON no reinício; o digest é detecção de corrupção, não autenticação. [`nexus/security.rs`](../../rullst-nexus/src/nexus/security.rs) mostra eventos locais como não assinados e mantém a audit chain em `Unavailable` até existir uma fonte verificadora. Testes cobrem timestamps/proveniência/contagem, spool público, reinício, quota, concorrência, symlink, adulteração e corrupção, além de `event_integrity_badge_never_promotes_unsigned_events`. |
| **P2-03 — honeypot confia XFF/substrings/bans eternos** | **Corrigido** | [`honey/middleware.rs`](../../rullst-security/src/honey/middleware.rs) usa peer `ConnectInfo`, paths exatos, TTL e limites de cardinalidade. Testes: `traps_use_exact_paths_and_bans_expire`, `ban_cardinality_is_bounded_and_invalid_ips_are_ignored` e `middleware_ignores_forwarded_identity_and_uses_socket_peer`. |
| **P2-04 — `TrafficShield::new` faz spawn/pode panic** | **Corrigido** | [`resilience.rs`](../../rullst-core/src/resilience.rs) separa construção de `start`, retorna erro fora de runtime e possui ownership/shutdown/drop que aborta monitores. Regressões incluem construção sem runtime, shutdown compartilhado e `dropping_final_shield_aborts_monitor_tasks`. |
| **P2-05 — scheduler sobrepõe jobs sem limite** | **Corrigido** | [`scheduler.rs`](../../rullst-core/src/scheduler.rs) serializa ticks por job, impõe timeout, converte panic em erro/política de falha e oferece shutdown/drop. Testes cobrem start fallible, timeout/panic, não sobreposição e `shutdown_aborts_current_handler`. |
| **P2-06 — queue spawn ilimitado/estado preso/JSON `null`** | **Corrigido** | [`queue/worker.rs`](../../rullst-core/src/queue/worker.rs) limita concorrência, timeout e shutdown, contém panic, torna transições observáveis e recupera jobs stalled; [`queue/sqlite.rs`](../../rullst-core/src/queue/sqlite.rs) falha JSON inválido e, com [`queue/redis.rs`](../../rullst-core/src/queue/redis.rs), persiste due times sem claim antecipado. Regressões em [`worker_tests.rs`](../../rullst-core/src/queue/worker_tests.rs), [`queue/tests.rs`](../../rullst-core/src/queue/tests.rs) e no contrato Redis live [`queue_scheduling_live.rs`](../../rullst-core/tests/queue_scheduling_live.rs). |
| **P2-07 — unload de dylib em uso/null para `Box::from_raw`** | **Corrigido** | [`server/hotswap.rs`](../../rullst-core/src/server/hotswap.rs) retém handles até o shutdown, evitando unload com request em voo; [`dylib_loader.rs`](../../rullst-core/src/server/dylib_loader.rs) converte o retorno a `NonNull` antes de `Box::from_raw` e documenta o contrato FFI. O hot reload continua uma fronteira `unsafe` dev-only baseada em ABI Rust e requer revisão dedicada, não uma garantia geral de segurança de memória. |
| **P2-08 — `env::set_var` dentro do runtime** | **Corrigido** | [`server/builder.rs`](../../rullst-core/src/server/builder.rs) lê dotenv em mapa local e resolve configuração sem mutar o ambiente global. Ocorrências restantes de `set_var` estão em fixtures/testes controlados. |
| **P2-09 — replicação de DB simulada** | **Mitigado / fail-closed** | [`db.rs`](../../rullst-core/src/db.rs) retorna `ReplicationError::Unsupported` quando há `sync_url`, em vez de logar sucesso fictício; `test_replication_manager_start` fixa esse contrato. Replicação real/vendor-specific não foi implementada. |
| **P2-10 — placeholders SQL por replace textual** | **Corrigido** | [`pool/placeholders.rs`](../../rullst-orm/src/pool/placeholders.rs) possui lexer consciente de strings, identificadores, comentários, dollar quotes, parâmetros existentes e operadores JSON. Testes `preserves_quoted_text_comments_and_dollar_quotes` e `preserves_json_operators_and_continues_existing_parameters` protegem os casos citados. Não é um parser SQL completo para todo dialeto futuro. |
| **P2-11 — identificador aceita hífen** | **Corrigido** | [`schema/validation.rs`](../../rullst-orm/src/schema/validation.rs) restringe componentes a alfanumérico/underscore e no máximo um separador de qualificação; hífen e formas ambíguas são rejeitados pelos testes do módulo. |
| **P2-12 — auditoria ORM perde diff/segredo aninhado** | **Corrigido** | [`audit/diff.rs`](../../rullst-orm/src/audit/diff.rs) calcula diferenças recursivas, mascara objetos/arrays aninhados, preserva primitivos/arrays e registra sentinela para JSON inválido. Testes `nested_secrets_are_redacted_in_objects_and_arrays` e `invalid_and_non_object_json_changes_are_not_dropped`. |
| **P2-13 — TOTP abreviado/URI com escape HTML** | **Corrigido** | [`mfa.rs`](../../rullst-security/src/mfa.rs) exige seis dígitos ASCII, compara de forma constante e usa percent-encoding no URI. [`recovery_codes.rs`](../../rullst-security/src/recovery_codes.rs) acrescenta verificadores subject-bound salted/HMAC e consumo único explícito; persistência atômica continua da aplicação. Testes: `totp_requires_exactly_six_ascii_digits`, `test_otpauth_uri_builder` e `codes_are_subject_bound_single_use_and_do_not_store_plaintext`. |
| **P2-14 — Auth gerado bloqueia Tokio/timing/busca linear** | **Corrigido** | [`auth/controllers.rs`](../../cargo-rullst/src/generators/auth/controllers.rs) gera lookup indexado, hash/verify via `spawn_blocking`, dummy hash para usuário ausente e não emite calls panicking. Regressão: `generated_auth_is_async_query_bound_and_panic_free`; o modelo gerado também cria índice/constraint de email. |
| **P2-15 — JWT gerado incompleto/versão divergente** | **Corrigido** | [`cors_jwt.rs`](../../cargo-rullst/src/generators/cors_jwt.rs) injeta dependência idempotente do workspace, exige segredo forte e valida claims `iss`, `aud`, `sub`, `iat` e `exp` por configuração tipada. Regressões: `generated_jwt_validates_secret_issuer_and_audience` e teste de dependências atuais/idempotentes. |
| **P2-16 — macro pública `#[route]` incompleta** | **Mitigado / fail-closed** | [`rullst-macros/src/lib.rs`](../../rullst-macros/src/lib.rs) mantém somente marcador de compatibilidade deprecado: atributo vazio preserva a função e atributo que fingiria registrar rota produz erro orientando `routes!`. [`route_compat.rs`](../../rullst-macros/tests/route_compat.rs) fixa a compatibilidade. Não existe registro funcional por `#[route]`; remoção em major futura ainda é trabalho de contrato. |
| **P2-17 — sinais semver incompletos** | **Parcial** | Vários erros/configs centrais agora usam `#[non_exhaustive]`, APIs antigas têm `#[deprecated]`, e [`.github/workflows/semver.yml`](../../.github/workflows/semver.yml) enumera todos os pacotes públicos publicados. A aplicação não é uniforme em toda a superfície pública; structs/enums históricos e política de transição ainda exigem inventário por crate. |
| **P2-18 — ergonomia/panic de constructors inconsistente** | **Parcial** | Connect, AI, Capital, Mail, IoT e os caminhos identificados em Core/ORM adotaram `impl Into<String>` e/ou construção fallible; `Column`, `JoinClause`, `RawExpression`, secrets, cache e queues também aceitam strings owned, e joins inválidos registram erro em vez de produzir SQL inseguro. A superfície histórica completa ainda precisa de inventário SemVer e migração gradual de builders/adapters restantes. |
| **P2-19 — `mutants` como dependência runtime do ORM** | **Corrigido** | [`rullst-orm/Cargo.toml`](../../rullst-orm/Cargo.toml) moveu `mutants` para `[dev-dependencies]`; `cfg(mutants)` é declarado ao lint sem carregar tooling em consumidores. |
| **P2-20 — compliance gerado imprime PASS incondicional** | **Corrigido** | [`audit_compliance.rs`](../../cargo-rullst/src/generators/audit_compliance.rs) modela `NoFindings`, `Findings`, `Generated`, `Observed`, `NotChecked` e `Error`, descreve o limite da evidência e rejeita linguagem de certificação. Teste: `report_never_fabricates_compliance_passes`. |
| **P2-21 — Basic Auth Nexus sem rate limit/user constant-time/TLS** | **Corrigido** | [`access.rs`](../../rullst-nexus/src/nexus/access.rs) compara username e senha em tempo constante, exige marcador TLS confiável e aplica limiter por peer. Regressões em [`access/tests.rs`](../../rullst-nexus/src/nexus/access/tests.rs): `basic_credentials_require_both_exact_values`, `basic_auth_requires_verified_tls` e `basic_auth_locks_peer_after_bounded_failures`. |

## Roadmap recomendado do §15 — Fases 0 a 4

Esta seção avalia o critério agregado de cada passo do §15, não reclassifica os
achados individuais.

### Fase 0 — contenção imediata

| Passo | Estado | Evidência/pendência |
|---|---|---|
| **0.1 Conter Fiscal, IoT crypto/MQTT, Vault, S3/R2 e Alipay** | **Mitigado / fail-closed** | Vault e OTA foram implementados; NFS-e/Alipay/remote storage falham fechados; simuladores IoT são explícitos. As integrações reais permanecem no capability ledger/ROADMAP. |
| **0.2 Nexus fechado; remover credenciais geradas** | **Corrigido** | `Nexus::try_build` exige policy, legacy é deny-all e blueprints não incluem `admin/password`; ver P0-04. |
| **0.3 Corrigir CORS e avisar projetos já scaffoldados** | **Corrigido** | O template e seus testes foram corrigidos (P0-05), e o [`cors-scaffold-security-advisory.md`](cors-scaffold-security-advisory.md) fornece detecção, correção e validação para projetos já scaffoldados. |
| **0.4 Corrigir traversal do Storage** | **Corrigido** | Validação, canonicalização e teste de symlink em `storage.rs`; ver P0-06. |
| **0.5 Unificar ambiente/fail-closed no startup** | **Corrigido** | `Environment` único, precedência e secure defaults testados; ver P0-07. |
| **0.6 Segredo obrigatório em webhook real** | **Mitigado / fail-closed** | Segredos vazios/mock não entram no modo real; assinatura/freshness/replay existem. Falta store de idempotência cross-instance; ver P0-08/P1-04. |
| **0.7 Corrigir e bloquear release** | **Corrigido** | Workflow tag-only com preflight/package-all/DAG/checksum/attestation; ver P0-09. Nenhuma execução é inferida da leitura do YAML. |
| **0.8 Alinhar README/SST/AUDIT/compliance** | **Corrigido** | [`README.md`](../../README.md), [`spec.md`](spec.md), [`AUDIT.md`](../../AUDIT.md), [`rullst-connect/AUDIT.md`](../../rullst-connect/AUDIT.md) e [`SECURITY_COMPLIANCE.md`](../../SECURITY_COMPLIANCE.md) distinguem implementação, mock, fail-closed e roadmap. O ledger é a matriz canônica de claims. |

### Fase 1 — segurança e confiabilidade do kernel

| Passo | Estado | Evidência/pendência |
|---|---|---|
| **1.1 Enum de ambiente e precedência** | **Corrigido** | P0-07. |
| **1.2 DB init atômica/fallible; getters normais sem panic** | **Corrigido** | P1-08. |
| **1.3 APP_KEY e sessão legacy** | **Corrigido** | P1-02. |
| **1.4 WebAuthn auditável/conforme** | **Parcial** | Invariantes listadas foram corrigidas e têm negativos, mas falta suíte normativa/biblioteca auditada; P1-01. |
| **1.5 DLP/PII por content-type/stream/header** | **Corrigido** | P1-03. |
| **1.6 CSRF separado de webhook + freshness/idempotência** | **Mitigado / fail-closed** | Composição, freshness e replay local corrigidos; idempotência compartilhada permanece; P1-04. |
| **1.7 Login Guard, rate, proxy, tenant e CSWSH** | **Corrigido no boundary definido** | Os bypasses/limites locais foram corrigidos e `rullst-security/redis-rate-limit` oferece contador Redis atômico opt-in, com fallback mock explícito, startup fail-closed e teste de duas instâncias contra Redis real. Failover/cluster e composição da identidade/origem no HTTP continuam gates de integração. P1-09/P1-10/P1-18/P1-19. |
| **1.8 Zero-panic em produção e código gerado** | **Corrigido no escopo declarado** | Os caminhos enumerados foram corrigidos; o gate inclui bibliotecas, macros, CLI/binários, Wasm e auditoria genérica dos templates runtime (P1-17). Não é uma promessa impossível sobre dependências, OOM ou falhas do host. |

### Fase 2 — integridade de produto e scaffolding

| Passo | Estado | Evidência/pendência |
|---|---|---|
| **2.1 Harness de todos os comandos + fmt/check/smoke** | **Parcial** | Há matriz estrutural de 270 combinações, materialização dos seis blueprints, parse de templates e inventário de todos os comandos; dois projetos passam `cargo check` real. `fmt/check/smoke` de toda combinação aplicável e comandos externos em ambientes de contrato continuam pendentes; P1-13. |
| **2.2 Flags, nomes, paths, IDs, Auth, Billing e Docs SSG** | **Corrigido** | Os bugs concretos têm regressões em `cargo-rullst/src` e os dois projetos gerados compilam no harness focado. |
| **2.3 Nexus server-side RBAC/ownership/field policy** | **Corrigido** | Role layer e field policy são server-side; batch/errors/escaping foram corrigidos; P0-04/P1-11. A identidade/membership final continua responsabilidade da aplicação hospedeira. |
| **2.4 Studio: rotas, escaping, redaction e métricas** | **Corrigido** | P1-12. |
| **2.5 Mocks offline AI/Mail/Connect sem fail-open real** | **Corrigido** | P1-14/P1-15/P1-16 e suites offline focadas. |

### Fase 3 — arquitetura e contrato

| Decisão/ação | Estado | Evidência/pendência |
|---|---|---|
| **Escolher Estratégia A ou B** | **Corrigido** | A SST e o [capability ledger](capability-ledger.md) adotam explicitamente o resultado prático da **Estratégia B**: Connect é OAuth/OIDC; messaging, NFS-e real, transports MQTT/CoAP, HSM/PQC e outros providers permanecem roadmap/fail-closed. |
| **Desacoplar Core de ORM** | **Corrigido no boundary definido** | [`rullst-core/Cargo.toml`](../../rullst-core/Cargo.toml) é runtime-only por default; `orm` e `queue-sqlite` são bridges opt-in independentes, com boundaries em CI. A dependência opcional é integração explícita, não acoplamento do Core mínimo. |
| **Consolidar Security** | **Parcial** | Claims e composição foram alinhados, e Core/Security reutilizam o mesmo `CspNonce`, evitando CSPs conflitantes. Ainda há ownership/headers/WAF/PII no Core e headers/RASP/DLP no Security. Uma dependência direta Core → Security criaria ciclo; o caminho seguro é um trait de stack no Core, implementação no crate dedicado e deprecation gradual das duplicações. |
| **Completar umbrella** | **Corrigido no contrato atual** | [`rullst/Cargo.toml`](../../rullst/Cargo.toml) possui features/reexports explícitos para Security, IoT, Connect, SMTP e boundaries mínimos testados. Capacidades ausentes continuam roadmap, sem reexport fictício. |
| **Padronizar semver/builders/`Into<String>`** | **Parcial** | P2-17/P2-18. |

### Fase 4 — engenharia de release

| Passo | Estado | Evidência/pendência |
|---|---|---|
| **4.1 Tríade AGENTS como gate `--all-features`** | **Corrigido** | [`ci.yml`](../../.github/workflows/ci.yml) e [`release.yml`](../../.github/workflows/release.yml) executam fmt, Clippy workspace/all-targets/all-features com `-D warnings` e testes workspace/all-features. A tríade também passou localmente no worktree de 2026-08-28; uma execução de CI/tag continua evidência separada. |
| **4.2 Features strict DB isoladas** | **Corrigido no workflow** | O job `strict-database-features` de [`ci.yml`](../../.github/workflows/ci.yml) compila e executa CRUD específico em `strict-postgres`, `strict-mysql` e `strict-sqlite`, cada um somente com sua feature; boundary mínimo Core/umbrella também é exercitado. |
| **4.3 Unsafe/WASM/Kani/Miri/mutation honestos** | **Corrigido no contrato automatizado** | [`unsafe-policy.yml`](../../.github/workflows/unsafe-policy.yml) e [`wasm-matrix.yml`](../../.github/workflows/wasm-matrix.yml) são bloqueantes; Kani, Miri, mutants e udeps são explicitamente informativos nos YAMLs e em [`WORKFLOWS.md`](../../WORKFLOWS.md). |
| **4.4 Cobrir 40 fuzz targets ou documentar tiers** | **Corrigido no workflow** | Há 40 arquivos em `*/fuzz/fuzz_targets`; a matriz manual [`fuzzing.yml`](../../.github/workflows/fuzzing.yml) enumera os 40 e [`WORKFLOWS.md`](../../WORKFLOWS.md) registra o limite: configuração não equivale a uma campanha executada com sucesso. |
| **4.5 Package-all antes do primeiro publish** | **Corrigido** | `release.yml` usa `cargo package --workspace ... --all-features --locked` no job `verify`, antes do job `publish`; P0-09. |
| **4.6 SBOM/audit/compliance por tag + digest/assinatura** | **Corrigido no workflow** | A release tag-only agrega metadata/Cargo.lock, Cargo Audit com exceções governadas, SBOM CycloneDX, relatório de evidência limitado, policy/advisory ledger, checksums e contexto tag/commit; os `.crate` e o bundle são atestados e anexados. Isso não é certificação nem prova de uma execução verde ainda não observada. |
| **4.7 Alinhar 12.0.0/changelog/tag/crates/release notes** | **Parcial** | Todos os manifests publicáveis inspecionados estão em `12.0.0` e o release rejeita tag divergente. [`CHANGELOG.md`](../../CHANGELOG.md) ainda marca 12.0.0 como Unreleased e o `HEAD` inspecionado não possui tag exata; crates.io/release notes não foram verificadas por esta inspeção local. |

## Evidência de testes e limites da validação

Há três tipos de evidência neste documento:

1. **Evidência estática local**: arquivos de implementação e testes nomeados nas
   tabelas foram inspecionados no worktree atual.
2. **Execuções focadas registradas durante o hardening**: Connect, AI, Mail,
   CSWSH/Security, tenant/Core, macros ORM, cache global, CSP compartilhado,
   scaffolding e strict-SQLite tiveram regressões focadas. A matriz estrutural
   validou 270 combinações e sete projetos representativos cobrindo os seis
   blueprints passaram checks Cargo offline, inclusive um em release.
3. **Tríade local final reexecutada em 2026-08-28**, com Rust/Cargo 1.98.0:

```text
cargo test --workspace --all-features                                PASS
cargo clippy --workspace --all-targets --all-features -- -D warnings PASS
cargo fmt --all -- --check                                           PASS
cargo +1.96.0 check --workspace --all-features                       PASS
.github/check-feature-boundaries.sh                                  PASS
.github/check-threat-model-release-minimum.sh                        PASS
.github/check-ai-evals.sh                                            PASS
.github/check-crates-ownership.sh                                    PASS
git diff --check                                                     PASS
```

O teste workspace/all-features foi executado sobre o worktree depois da
auditoria dos exemplos e incluiu os doc-tests. Nove dos dez doctests antes
marcados `ignore` agora compilam; o único restante está justificado na crate
proc-macro e tem cobertura equivalente na facade. O teste de driver de banco
ignorado sob a combinação artificial `--all-features` continua exigindo as
matrizes exclusivas por driver. A dependência do Swagger UI foi posteriormente
alterada para o bundle vendorizado, removendo o download GitHub/DNS durante
builds limpos.

Os testes de integração que abrem loopback foram executados fora do sandbox
restritivo; o restante permaneceu local. A dependência transitiva
`proc-macro-error2 2.0.1` citada na execução histórica foi posteriormente
removida junto com a dependência Leptos não utilizada de Connect.

As matrizes CRUD exclusivas `strict-sqlite`, `strict-postgres` e `strict-mysql`
de `rullst-orm` passaram localmente em 2026-08-28; PostgreSQL e MySQL usaram
Testcontainers Docker isolados. Essa evidência é delimitada ao ORM e ao worktree,
não ao workspace/Academy completo nem à futura tag. Não foram executadas aqui
campanhas de fuzz, Kani, Miri, mutation, DAST, scanners de dependência,
package/publish, uma pipeline de tag ou homologações externas. Arquivos YAML não
constituem, por si sós, resultado.

## Lacunas técnicas remanescentes

Em ordem de risco/impacto, o código e a governança ainda precisam de:

1. backend compartilhado e atômico para idempotência de webhooks/tracking e
   testes Redis reais multi-instância/failover para o rate limiter disponível;
2. suíte normativa/biblioteca auditada para WebAuthn, além dos negativos atuais;
3. matriz de geração cobrindo todos os comandos/blueprints com fmt/check/smoke;
4. conclusão da arquitetura Security da Fase 3: um contrato de stack no Core,
   implementação canônica no crate dedicado e deprecation das duplicações;
5. inventário semver e migração uniforme de constructors/builders fallible;
6. revisão específica da fronteira FFI/ABI do hot reload dev-only, incluindo
   testes apropriados de ciclo de vida; retenção de dylibs corrige o unload
   inseguro, mas não transforma ABI Rust dinâmica em contrato estável;
7. atualizar ou substituir a cadeia opcional Leptos que ainda traz
   `proc-macro-error2 2.0.1`, antes que o lint futuro E0365 vire erro do Rust;
8. execução verde da pipeline de tag no commit/release exato, alinhando versão,
   changelog, tag, crates.io e release notes;
9. somente quando houver mantenedor, ambiente de interoperabilidade e suite de
   contrato: NFS-e homologada, Alipay RSA2, storage remoto, replicação específica,
   messaging, MQTT, HSM/PQC e ciclo real de flash/boot. Até lá, manter
   `Unsupported`/experimental é o comportamento seguro.
