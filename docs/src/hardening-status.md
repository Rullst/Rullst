# Hardening status — rastreabilidade da avaliação `gpt.md`

> Estado pontual do worktree em **2026-08-24**, baseado no `HEAD`
> `da4057a3f11e` e nas alterações locais ainda não consolidadas. Este documento
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
| P1-01..19 | 15 | 2 | 2 | 0 |
| P2-01..21 | 17 | 2 | 2 | 0 |
| **Total dos 49 achados** | **38** | **7** | **4** | **0** |

“Sem pendência” nessa tabela significa que cada achado possui ao menos uma
correção ou contenção; não significa que as capacidades fail-closed foram
implementadas. NFS-e real, Alipay RSA2, storage remoto, replicação genérica,
MQTT/HSM/PQC e rate limiting distribuído continuam fora do contrato entregue,
como registra o capability ledger.

## Achados críticos — P0

| ID | Estado | Evidência atual e limite |
|---|---|---|
| **P0-01 — NFS-e/XMLDSig inválida** | **Mitigado / fail-closed** | [`fiscal/signer.rs`](../../rullst-capital/src/fiscal/signer.rs) retorna `FiscalError::Unsupported`; [`fiscal/client.rs`](../../rullst-capital/src/fiscal/client.rs) diferencia `Mock`, `Homologation` e `Production`, e os modos reais falham fechados. Regressões: `explicit_mock_is_distinguishable_and_deterministic` e `real_environments_fail_closed_even_with_mock_or_empty_certificate`. XMLDSig, C14N, mTLS e homologação SEFIN continuam não implementados. |
| **P0-02 — `FieldEncryptor` irreversível/falso** | **Corrigido** | [`vault.rs`](../../rullst-security/src/vault.rs) usa envelope versionado AES-256-GCM, nonce aleatório, AAD e key-id/keyring, rejeitando chave de tamanho incorreto e envelope legado ambíguo. Round-trip, adulteração e chave incorreta são cobertos em [`mfa_sri_vault_test.rs`](../../rullst-security/tests/mfa_sri_vault_test.rs). |
| **P0-03 — stubs criptográficos IoT expostos como garantias** | **Mitigado / fail-closed** | [`ota.rs`](../../rullst-iot/src/ota.rs) verifica assinatura Ed25519 de manifesto canônico, hash/tamanho/target e anti-rollback antes de permitir commit; APIs legadas falham com erro tipado. MQTT/HSM/PQC foram renomeados para `Simulated*` e isolados por `experimental-simulators` em [`lib.rs`](../../rullst-iot/src/lib.rs) e [`Cargo.toml`](../../rullst-iot/Cargo.toml). Os vetores OTA e a contenção de APIs legadas estão em [`iot_integration_tests.rs`](../../rullst-iot/tests/iot_integration_tests.rs). Transporte, hardware e ML-KEM reais continuam roadmap. |
| **P0-04 — Nexus destrutivo aberto por padrão** | **Corrigido** | [`nexus/mod.rs`](../../rullst-nexus/src/nexus/mod.rs) exige política de autenticação em `try_build`, devolve `MissingAuthenticationPolicy` sem ela e instala `RequireRoleLayer<NexusPrincipal>`; construtores legados são deny-all/deprecados. [`access.rs`](../../rullst-nexus/src/nexus/access.rs) exige credenciais fortes, peer rate limit e fronteira TLS verificável. Testes: [`access/tests.rs`](../../rullst-nexus/src/nexus/access/tests.rs) e casos de build fail-closed no próprio módulo. |
| **P0-05 — CORS reflect-origin gerado** | **Corrigido** | [`cors_middleware.rs.template`](../../cargo-rullst/src/generators/cors_middleware.rs.template) gera allowlist tipada, rejeita wildcard/origens inválidas, não habilita credenciais por padrão e limita métodos. Regressões do gerador estão em [`cors_jwt.rs`](../../cargo-rullst/src/generators/cors_jwt.rs). A etapa separada de aviso a consumidores antigos é tratada na Fase 0 abaixo. |
| **P0-06 — storage confirma persistência inexistente/path traversal** | **Corrigido** | [`storage.rs`](../../rullst-core/src/storage.rs) valida componentes, confina o caminho ao diretório canônico, barra escape por symlink e retorna `Unsupported` para S3/R2/resize. Testes: `relative_path_validation_rejects_escape_attempts`, `local_storage_rejects_symlink_escape`, `cloud_backends_never_report_false_success` e `resize_never_returns_unmodified_bytes_as_success`. |
| **P0-07 — ambiente de produção fail-open/inconsistente** | **Corrigido** | [`config.rs`](../../rullst-core/src/config.rs) define `Environment` e precedência validada `RULLST_ENV` → `APP_ENV` → config; valores inválidos são erros. [`server/builder.rs`](../../rullst-core/src/server/builder.rs) carrega dotenv sem mutar o ambiente global, propaga falhas de config/DB e instala defaults seguros em staging/produção. Regressão: `environment_resolution_has_one_precedence_and_validated_aliases`. |
| **P0-08 — integridade de providers/webhooks** | **Mitigado / fail-closed** | [`providers/mod.rs`](../../rullst-capital/src/providers/mod.rs) rejeita segredo real vazio e separa mock explícito; verificadores com timestamp aplicam freshness. [`webhook.rs`](../../rullst-capital/src/webhook.rs) adiciona replay store limitado e modo produção que rejeita mock. [`alipay.rs`](../../rullst-capital/src/providers/alipay.rs) retorna `UnsupportedOperation` no modo RSA2 real, em vez de simular HMAC como RSA. Regressões incluem `explicit_mock_webhook_mode_still_requires_its_secret`, `replay_store_rejects_duplicates_and_expires_entries` e `replay_store_is_bounded_and_provider_scoped`. O replay store ainda é em memória e não resolve idempotência entre processos. |
| **P0-09 — release fora da ordem topológica** | **Corrigido** | [`.github/workflows/release.yml`](../../.github/workflows/release.yml) verifica fmt/Clippy/testes, empacota todos os crates antes do primeiro publish e publica pela DAG real, incluindo Connect/IoT e ORM antes de Core; valida tag/versões, checksums e artefatos. É evidência estrutural do workflow, não evidência de que uma execução de release passou. |

## Achados altos — P1

| ID | Estado | Evidência atual e limite |
|---|---|---|
| **P1-01 — invariantes WebAuthn incompletas** | **Parcial** | [`passkey/service.rs`](../../rullst-auth/src/auth/passkey/service.rs) e [`ceremony.rs`](../../rullst-auth/src/auth/passkey/ceremony.rs) validam tipo da cerimônia, origin/rpIdHash, cross-origin, UP/UV, COSE ES256, raw-id, contador monotônico e challenges compartilhados, bounded, one-time e com TTL. Os negativos estão em [`invariant_tests.rs`](../../rullst-auth/src/auth/passkey/invariant_tests.rs). O escopo de attestation permanece deliberadamente estreito (`none`/ES256) e ainda não há prova por suíte normativa ou biblioteca WebAuthn auditada; o ledger mantém “Full normative WebAuthn server” como parcial. |
| **P1-02 — sessão/APP_KEY/rehash/legado** | **Corrigido** | [`auth.rs`](../../rullst-auth/src/auth.rs) valida força/placeholder do segredo nos ambientes seguros, compara algoritmo/versão/parâmetros Argon2 para rehash e só aceita envelope de sessão versionado e expirável. Regressões incluem `unversioned_session_tokens_are_rejected` e testes de `needs_rehash`/expiração. O JWT de aplicação continua explicitamente responsabilidade do scaffold, conforme o ledger. |
| **P1-03 — DLP/PII corrompem respostas** | **Corrigido** | [`dlp.rs`](../../rullst-security/src/dlp.rs) e [`pii.rs`](../../rullst-core/src/security/pii.rs) só reescrevem texto bufferizável suportado; binário, streaming, encoding e overflow têm bypass/erro explícito sem UTF-8 lossy ou falso sucesso, e headers de representação são recalculados/removidos. Regressões: `invalid_utf8_and_incomplete_pem_are_not_corrupted`, `test_dlp_layer_middleware` e [`core/security/tests.rs`](../../rullst-core/src/security/tests.rs). |
| **P1-04 — CSRF incompatível com webhooks** | **Mitigado / fail-closed** | [`csrf.rs`](../../rullst-core/src/security/csrf.rs) reconhece métodos seguros e apenas isenções POST exatas configuradas; o scaffold monta a rota de webhook assinada separadamente em [`saas/routes.rs`](../../cargo-rullst/src/blueprints/saas/routes.rs). Capital exige assinatura/freshness/replay. Testes: `safe_http_methods_do_not_require_a_token` e `only_exact_configured_post_webhook_path_is_exempt`. A idempotência cross-instance ainda requer backend compartilhado. |
| **P1-05 — WAF/RASP não inspecionam body** | **Corrigido** | [`core/security/waf.rs`](../../rullst-core/src/security/waf.rs) e [`security/rasp.rs`](../../rullst-security/src/rasp.rs) inspecionam bodies text/JSON/form limitados, decodificam strings JSON, falham fechados em tamanho/encoding inválido e reconstroem a request. Regressões: `waf_inspects_and_preserves_bounded_request_bodies`, `json_body_inspection_decodes_escaped_strings` e testes de ataque/overflow no módulo RASP. Continuam heurísticas de defesa em profundidade, não substitutos para parser, bind ou autorização. |
| **P1-06 — CSP não corresponde a nonces/A+** | **Corrigido** | [`core/security/headers.rs`](../../rullst-core/src/security/headers.rs) e [`security/headers.rs`](../../rullst-security/src/headers.rs) emitem baseline sem `unsafe-inline`/`unsafe-eval` e expõem `CspNonce` ao renderer; policy estática continua presente quando nonce dinâmico está desligado. Testes verificam que nonce do header e extension coincidem em [`core/security/tests.rs`](../../rullst-core/src/security/tests.rs) e [`security_tests.rs`](../../rullst-security/tests/security_tests.rs). A documentação não deve prometer nota universal de scanner. |
| **P1-07 — middleware de webhook perde request parts/body** | **Corrigido** | [`webhook.rs`](../../rullst-capital/src/webhook.rs) usa `into_parts`/`from_parts`, mantém método, URI, versão, headers, extensions e bytes originais, além do evento verificado. Regressão: `reconstructed_request_preserves_parts_extensions_and_body`. |
| **P1-08 — inicialização ORM global parcial/panicking** | **Corrigido** | [`pool.rs`](../../rullst-orm/src/pool.rs) prepara primária/réplicas localmente com `try_join_all` e só então publica um único `OnceLock<OrmState>`; getters normais retornam `Result`. [`atomic_init_test.rs`](../../rullst-orm/tests/atomic_init_test.rs) prova que falha de réplica não publica estado parcial. |
| **P1-09 — janitor limpa clones do Login Guard** | **Corrigido** | [`login_guard.rs`](../../rullst-security/src/login_guard.rs) limpa diretamente os `DashMap` ativos via `cleanup_if_due`, aplica TTL e `max_identities`, sem task dona de clones divergentes. Testes `test_login_guard_tarpit_and_jail`, `test_login_guard_global_and_expired_jail` e concorrência em [`concurrency_tests.rs`](../../rullst-security/tests/concurrency_tests.rs). |
| **P1-10 — distributed rate limit no-op/IP forjável** | **Mitigado / fail-closed** | [`rate_limit.rs`](../../rullst-security/src/rate_limit.rs) retorna `DistributedBackendUnsupported` em vez de rotular o mapa local como distribuído e deriva peer de `ConnectInfo`. [`resilience.rs`](../../rullst-core/src/resilience.rs) ignora forwarded headers no extractor padrão e permite política explícita de proxy confiável. Testes: `test_default_key_extractor` e [`deep_security_coverage_tests.rs`](../../rullst-security/tests/deep_security_coverage_tests.rs). Backend Redis/atômico distribuído ainda não existe. |
| **P1-11 — XSS/autorização visual no Nexus** | **Corrigido** | [`ai_chat.rs`](../../rullst-nexus/src/nexus/ai_chat.rs) sanitiza saída externa; [`crud/views.rs`](../../rullst-nexus/src/nexus/crud/views.rs) escapa/encodeia IDs sem JS inline; [`crud/handlers.rs`](../../rullst-nexus/src/nexus/crud/handlers.rs) aplica `hidden`/`readonly`, limita batch e propaga erro; paginação é limitada/saturating. O router inteiro exige papel admin em [`nexus/mod.rs`](../../rullst-nexus/src/nexus/mod.rs). Testes de acesso e CRUD estão em [`rullst-nexus/tests`](../../rullst-nexus/tests). |
| **P1-12 — Studio inventa métricas/rota/XSS/env leak** | **Corrigido** | [`radar.rs`](../../rullst-core/src/radar.rs) representa probes ausentes como `Option`; [`security_radar.rs`](../../rullst-studio/src/security_radar.rs) usa `Unavailable`, rota coerente e DOM `textContent`/`replaceChildren`; [`env_viewer.rs`](../../rullst-studio/src/env_viewer.rs) usa política de redaction; [`feature_flags.rs`](../../rullst-studio/src/feature_flags.rs) encodeia nomes e usa binds corretos. Regressões: `test_studio_builder_and_routes`, `test_studio_security_radar_and_telemetry` e `test_studio_env_viewer_endpoint`. |
| **P1-13 — regressões concretas dos geradores** | **Parcial** | Flags, IDs estáveis, path/package, Island/Resource, Auth, Billing e Docs SSG têm correções/testes em [`cargo-rullst/src`](../../cargo-rullst/src). [`generated_saas_check.rs`](../../cargo-rullst/tests/generated_saas_check.rs) faz `cargo check --offline --all-targets` para SaaS e blank+Wasm Island. Falta a matriz pedida para **todos** os comandos/blueprints/combinações, com `cargo fmt --check`, `cargo check` e smoke tests. Templates grandes ainda permanecem inline em pontos do CLI. |
| **P1-14 — guardrails/mocks/DeepSeek/structured output** | **Corrigido** | [`guardrails.rs`](../../rullst-ai/src/ai/guardrails.rs) integra o estágio ao client de alto nível e providers repetem a proteção em chamadas diretas; [`deepseek.rs`](../../rullst-ai/src/ai/providers/deepseek.rs) existe; empty/`mock_*` é offline determinístico em chat/vision/embeddings. [`structured.rs`](../../rullst-ai/src/ai/structured.rs) diferencia JSON parseável de schema nativo e retorna `UnsupportedCapability` quando o provider não o garante. [`guardrails_pipeline_test.rs`](../../rullst-ai/tests/guardrails_pipeline_test.rs) percorre providers/capacidades e prova determinismo sem endpoint. Schema nativo em todos os LLMs continua visão, não claim. |
| **P1-15 — constructors/OIDC/JWKS de Connect** | **Corrigido** | Constructors gerados são fallible e aceitam `impl Into<String>` em [`macros.rs`](../../rullst-connect/src/macros.rs); credenciais mock/empty não podem ser redirecionadas a endpoint real. [`oidc/discovery.rs`](../../rullst-connect/src/providers/oidc/discovery.rs) valida URL/host/issuer/endpoints, e o client HTTP desabilita redirects. [`provider/jwks.rs`](../../rullst-connect/src/provider/jwks.rs) implementa TTL, refresh por `kid` desconhecido e stale limitado. Negativos/rotação estão em [`providers/oidc/tests.rs`](../../rullst-connect/src/providers/oidc/tests.rs) e [`provider/tests.rs`](../../rullst-connect/src/provider/tests.rs). |
| **P1-16 — invariantes de Mail/tenant/tracking** | **Corrigido** | [`pipeline.rs`](../../rullst-mail/src/pipeline.rs), [`facade.rs`](../../rullst-mail/src/facade.rs) e [`worker.rs`](../../rullst-mail/src/worker.rs) centralizam CRLF, deliverability, links e tenant; [`drivers/mock.rs`](../../rullst-mail/src/drivers/mock.rs) seleciona offline deterministicamente; [`tracking.rs`](../../rullst-mail/src/tracking.rs) exige segredo forte, HMAC constant-time, TTL e replay store limitado. Regressões estão em [`tracking_tests.rs`](../../rullst-mail/tests/tracking_tests.rs), [`pipeline_tests.rs`](../../rullst-mail/tests/pipeline_tests.rs) e [`mail_integration_tests.rs`](../../rullst-mail/tests/mail_integration_tests.rs). |
| **P1-17 — panics nos caminhos apontados** | **Corrigido** | Os caminhos citados foram convertidos a erros/fallbacks: client Wasm e server em [`rullst-core/src`](../../rullst-core/src), getters ORM em [`pool.rs`](../../rullst-orm/src/pool.rs), expansão ORM em [`rullst-orm-macros`](../../rullst-orm-macros/src) e Auth/Island gerados em [`cargo-rullst/src/generators`](../../cargo-rullst/src/generators). [`.github/workflows/zero-panics.yml`](../../.github/workflows/zero-panics.yml) lista exatamente os crates/caminhos cobertos e testa a expansão. Esta classificação vale para os exemplos de `gpt.md`; não é prova global de ausência de panic em todo artefato gerado, dependência ou runtime. |
| **P1-18 — tenant escolhido pelo cliente** | **Corrigido** | [`tenant_guard.rs`](../../rullst-core/src/security/tenant_guard.rs) deriva seleção de `TenantMembership`/`TenantContext` confiável e ignora headers como autoridade; [`multitenant.rs`](../../rullst-core/src/multitenant.rs) trata header/query/subdomínio apenas como seletor sujeito a membership, devolvendo 403 sem vínculo. Testes negativos estão nos próprios módulos e em [`rullst-core/tests`](../../rullst-core/tests). |
| **P1-19 — bypass CSWSH por prefixo de host** | **Corrigido** | [`cswsh.rs`](../../rullst-security/src/cswsh.rs) normaliza e compara esquema/host/porta exatos, com origin ausente fechado por padrão. Testes `deceptive_localhost_prefixes_are_rejected` e `middleware_rejects_a_deceptive_localhost_origin` cobrem `localhost.evil`. |

## Achados médios — P2

| ID | Estado | Evidência atual e limite |
|---|---|---|
| **P2-01 — audit chain ambígua/sem continuidade** | **Corrigido** | [`audit/chain.rs`](../../rullst-security/src/audit/chain.rs) usa serialização domain-separated e length-prefixed, chave forte, sequência atômica e commit somente após o logger; valida continuidade. Testes: `length_prefixing_prevents_delimiter_collisions`, `weak_keys_are_rejected` e `logger_failure_does_not_create_a_sequence_gap`. Persistência durável continua responsabilidade de um sink. |
| **P2-02 — telemetria finge HMAC/IP/tempo** | **Corrigido** | [`telemetry.rs`](../../rullst-security/src/telemetry.rs) força `verified_hmac=false` para evento local, não inventa loopback para IP inválido e emite RFC3339 absoluto. Testes: `timestamps_are_absolute_rfc3339_values` e `local_events_cannot_claim_hmac_verification_or_fake_ips`. |
| **P2-03 — honeypot confia XFF/substrings/bans eternos** | **Corrigido** | [`honey/middleware.rs`](../../rullst-security/src/honey/middleware.rs) usa peer `ConnectInfo`, paths exatos, TTL e limites de cardinalidade. Testes: `traps_use_exact_paths_and_bans_expire`, `ban_cardinality_is_bounded_and_invalid_ips_are_ignored` e `middleware_ignores_forwarded_identity_and_uses_socket_peer`. |
| **P2-04 — `TrafficShield::new` faz spawn/pode panic** | **Corrigido** | [`resilience.rs`](../../rullst-core/src/resilience.rs) separa construção de `start`, retorna erro fora de runtime e possui ownership/shutdown/drop que aborta monitores. Regressões incluem construção sem runtime, shutdown compartilhado e `dropping_final_shield_aborts_monitor_tasks`. |
| **P2-05 — scheduler sobrepõe jobs sem limite** | **Corrigido** | [`scheduler.rs`](../../rullst-core/src/scheduler.rs) serializa ticks por job, impõe timeout, converte panic em erro/política de falha e oferece shutdown/drop. Testes cobrem start fallible, timeout/panic, não sobreposição e `shutdown_aborts_current_handler`. |
| **P2-06 — queue spawn ilimitado/estado preso/JSON `null`** | **Corrigido** | [`queue/worker.rs`](../../rullst-core/src/queue/worker.rs) limita concorrência, timeout e shutdown, contém panic, torna transições observáveis e recupera jobs stalled; [`queue/sqlite.rs`](../../rullst-core/src/queue/sqlite.rs) falha JSON inválido. Regressões em [`worker_tests.rs`](../../rullst-core/src/queue/worker_tests.rs) e `invalid_json_is_an_error_and_the_job_is_failed` em [`queue/tests.rs`](../../rullst-core/src/queue/tests.rs). |
| **P2-07 — unload de dylib em uso/null para `Box::from_raw`** | **Corrigido** | [`server/hotswap.rs`](../../rullst-core/src/server/hotswap.rs) retém handles até o shutdown, evitando unload com request em voo; [`dylib_loader.rs`](../../rullst-core/src/server/dylib_loader.rs) converte o retorno a `NonNull` antes de `Box::from_raw` e documenta o contrato FFI. O hot reload continua uma fronteira `unsafe` dev-only baseada em ABI Rust e requer revisão dedicada, não uma garantia geral de segurança de memória. |
| **P2-08 — `env::set_var` dentro do runtime** | **Corrigido** | [`server/builder.rs`](../../rullst-core/src/server/builder.rs) lê dotenv em mapa local e resolve configuração sem mutar o ambiente global. Ocorrências restantes de `set_var` estão em fixtures/testes controlados. |
| **P2-09 — replicação de DB simulada** | **Mitigado / fail-closed** | [`db.rs`](../../rullst-core/src/db.rs) retorna `ReplicationError::Unsupported` quando há `sync_url`, em vez de logar sucesso fictício; `test_replication_manager_start` fixa esse contrato. Replicação real/vendor-specific não foi implementada. |
| **P2-10 — placeholders SQL por replace textual** | **Corrigido** | [`pool/placeholders.rs`](../../rullst-orm/src/pool/placeholders.rs) possui lexer consciente de strings, identificadores, comentários, dollar quotes, parâmetros existentes e operadores JSON. Testes `preserves_quoted_text_comments_and_dollar_quotes` e `preserves_json_operators_and_continues_existing_parameters` protegem os casos citados. Não é um parser SQL completo para todo dialeto futuro. |
| **P2-11 — identificador aceita hífen** | **Corrigido** | [`schema/validation.rs`](../../rullst-orm/src/schema/validation.rs) restringe componentes a alfanumérico/underscore e no máximo um separador de qualificação; hífen e formas ambíguas são rejeitados pelos testes do módulo. |
| **P2-12 — auditoria ORM perde diff/segredo aninhado** | **Corrigido** | [`audit/diff.rs`](../../rullst-orm/src/audit/diff.rs) calcula diferenças recursivas, mascara objetos/arrays aninhados, preserva primitivos/arrays e registra sentinela para JSON inválido. Testes `nested_secrets_are_redacted_in_objects_and_arrays` e `invalid_and_non_object_json_changes_are_not_dropped`. |
| **P2-13 — TOTP abreviado/URI com escape HTML** | **Corrigido** | [`mfa.rs`](../../rullst-security/src/mfa.rs) exige seis dígitos ASCII, compara de forma constante e usa percent-encoding no URI. Testes: `totp_requires_exactly_six_ascii_digits` e `test_otpauth_uri_builder`. |
| **P2-14 — Auth gerado bloqueia Tokio/timing/busca linear** | **Corrigido** | [`auth/controllers.rs`](../../cargo-rullst/src/generators/auth/controllers.rs) gera lookup indexado, hash/verify via `spawn_blocking`, dummy hash para usuário ausente e não emite calls panicking. Regressão: `generated_auth_is_async_query_bound_and_panic_free`; o modelo gerado também cria índice/constraint de email. |
| **P2-15 — JWT gerado incompleto/versão divergente** | **Corrigido** | [`cors_jwt.rs`](../../cargo-rullst/src/generators/cors_jwt.rs) injeta dependência idempotente do workspace, exige segredo forte e valida claims `iss`, `aud`, `sub`, `iat` e `exp` por configuração tipada. Regressões: `generated_jwt_validates_secret_issuer_and_audience` e teste de dependências atuais/idempotentes. |
| **P2-16 — macro pública `#[route]` incompleta** | **Mitigado / fail-closed** | [`rullst-macros/src/lib.rs`](../../rullst-macros/src/lib.rs) mantém somente marcador de compatibilidade deprecado: atributo vazio preserva a função e atributo que fingiria registrar rota produz erro orientando `routes!`. [`route_compat.rs`](../../rullst-macros/tests/route_compat.rs) fixa a compatibilidade. Não existe registro funcional por `#[route]`; remoção em major futura ainda é trabalho de contrato. |
| **P2-17 — sinais semver incompletos** | **Parcial** | Vários erros/configs centrais agora usam `#[non_exhaustive]`, APIs antigas têm `#[deprecated]`, e [`.github/workflows/semver.yml`](../../.github/workflows/semver.yml) audita crates públicos. A aplicação não é uniforme em toda a superfície pública; structs/enums históricos e política de transição ainda exigem inventário por crate. |
| **P2-18 — ergonomia/panic de constructors inconsistente** | **Parcial** | Connect, AI, Capital, Mail, IoT e vários tipos Core adotaram `impl Into<String>` e/ou `try_new`; caminhos perigosos foram tornados fallible. Permanecem constructors públicos históricos com `&str`/`String` rígido, por exemplo em [`schema/column.rs`](../../rullst-orm/src/schema/column.rs), [`schema/join.rs`](../../rullst-orm/src/schema/join.rs) e adapters antigos. Falta auditoria e migração uniforme com deprecation. |
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
| **0.3 Corrigir CORS e avisar projetos já scaffoldados** | **Parcial** | O template e seus testes foram corrigidos (P0-05), mas não foi encontrado advisory específico e versionado para consumidores de scaffolds antigos. [`security-advisory-exceptions.md`](security-advisory-exceptions.md) governa exceções RustSec, não substitui esse aviso de migração CORS. |
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
| **1.7 Login Guard, rate, proxy, tenant e CSWSH** | **Mitigado / fail-closed** | Todos os bypasses/limites locais foram corrigidos; distributed rate limit retorna Unsupported, sem backend real; P1-09/P1-10/P1-18/P1-19. |
| **1.8 Zero-panic em produção e código gerado** | **Parcial** | Os caminhos enumerados foram corrigidos e há gate CI com escopo declarado (P1-17), mas não existe prova global para todos os generators/configurações/dependências. |

### Fase 2 — integridade de produto e scaffolding

| Passo | Estado | Evidência/pendência |
|---|---|---|
| **2.1 Harness de todos os comandos + fmt/check/smoke** | **Parcial** | Existem checks reais para dois projetos gerados, não a matriz completa; P1-13. |
| **2.2 Flags, nomes, paths, IDs, Auth, Billing e Docs SSG** | **Corrigido** | Os bugs concretos têm regressões em `cargo-rullst/src` e os dois projetos gerados compilam no harness focado. |
| **2.3 Nexus server-side RBAC/ownership/field policy** | **Corrigido** | Role layer e field policy são server-side; batch/errors/escaping foram corrigidos; P0-04/P1-11. A identidade/membership final continua responsabilidade da aplicação hospedeira. |
| **2.4 Studio: rotas, escaping, redaction e métricas** | **Corrigido** | P1-12. |
| **2.5 Mocks offline AI/Mail/Connect sem fail-open real** | **Corrigido** | P1-14/P1-15/P1-16 e suites offline focadas. |

### Fase 3 — arquitetura e contrato

| Decisão/ação | Estado | Evidência/pendência |
|---|---|---|
| **Escolher Estratégia A ou B** | **Corrigido** | A SST e o [capability ledger](capability-ledger.md) adotam explicitamente o resultado prático da **Estratégia B**: Connect é OAuth/OIDC; messaging, NFS-e real, MQTT/HSM/PQC e outros providers permanecem roadmap/fail-closed. |
| **Desacoplar Core de ORM** | **Parcial** | [`rullst-core/Cargo.toml`](../../rullst-core/Cargo.toml) tornou Core runtime-only por default e isolou `orm`/`queue-sqlite`, com jobs de boundary em `ci.yml`; Core ainda possui dependência opcional e módulos de integração com ORM. |
| **Consolidar Security** | **Parcial** | Limites e claims foram alinhados, mas ainda existem implementações de headers/WAF/PII no Core e headers/RASP/DLP no Security. O ledger marca “One canonical security stack” como prioridade parcial. |
| **Completar umbrella** | **Parcial** | [`rullst/Cargo.toml`](../../rullst/Cargo.toml) possui features explícitas e boundary mínimo testado; a consolidação de contratos/reexports e o desacoplamento Core↔ORM ainda não terminaram. |
| **Padronizar semver/builders/`Into<String>`** | **Parcial** | P2-17/P2-18. |

### Fase 4 — engenharia de release

| Passo | Estado | Evidência/pendência |
|---|---|---|
| **4.1 Tríade AGENTS como gate `--all-features`** | **Corrigido** | [`ci.yml`](../../.github/workflows/ci.yml) e [`release.yml`](../../.github/workflows/release.yml) executam fmt, Clippy workspace/all-targets/all-features com `-D warnings` e testes workspace/all-features. A existência do gate não comprova o resultado do worktree atual. |
| **4.2 Features strict DB isoladas** | **Corrigido** | O job `strict-database-features` de [`ci.yml`](../../.github/workflows/ci.yml) compila `strict-postgres`, `strict-mysql` e `strict-sqlite` individualmente; boundary mínimo Core/umbrella também é exercitado. |
| **4.3 Unsafe/WASM/Kani/Miri/mutation honestos** | **Parcial** | [`unsafe-policy.yml`](../../.github/workflows/unsafe-policy.yml) e [`wasm-matrix.yml`](../../.github/workflows/wasm-matrix.yml) são bloqueantes; Kani/Miri/mutants são explicitamente não bloqueantes em seus YAMLs. Porém [`WORKFLOWS.md`](../../WORKFLOWS.md) ainda usa linguagem como “matematicamente prove”/“asserting every mutant” e precisa ser alinhado ao comportamento informativo. |
| **4.4 Cobrir 40 fuzz targets ou documentar tiers** | **Parcial** | Há 40 arquivos em `*/fuzz/fuzz_targets` e a matriz manual [`fuzzing.yml`](../../.github/workflows/fuzzing.yml) lista os 40. `WORKFLOWS.md` ainda documenta 33/33+ e não há execução desta matriz registrada nesta inspeção. |
| **4.5 Package-all antes do primeiro publish** | **Corrigido** | `release.yml` usa `cargo package --workspace ... --all-features --locked` no job `verify`, antes do job `publish`; P0-09. |
| **4.6 SBOM/audit/compliance por tag + digest/assinatura** | **Parcial** | Release gera `.crate`, SHA-256, attestation e provenance; audits/SBOM/compliance existem em outros workflows/CLI, mas não são gerados e anexados pelo fluxo tag-only como um único conjunto de evidência. |
| **4.7 Alinhar 12.0.0/changelog/tag/crates/release notes** | **Parcial** | Todos os manifests publicáveis inspecionados estão em `12.0.0` e o release rejeita tag divergente. [`CHANGELOG.md`](../../CHANGELOG.md) ainda marca 12.0.0 como Unreleased e o `HEAD` inspecionado não possui tag exata; crates.io/release notes não foram verificadas por esta inspeção local. |

## Evidência de testes e limites da validação

Há dois tipos de evidência neste documento:

1. **Evidência estática local**: arquivos de implementação e testes nomeados nas
   tabelas foram inspecionados no worktree atual.
2. **Execuções focadas registradas durante o hardening**: antes da limpeza dos
   artefatos de build, foram executados testes/Clippy focados para Connect; AI;
   Mail; CSWSH em Security; tenant em Core; e expansão de macros ORM. Entre os
   resultados registrados estão 183 testes unitários + 18 de integração em
   Connect, as suites offline de AI/Mail, os seis testes CSWSH e os testes de
   tenant/macro citados. Esses resultados sustentam apenas os respectivos
   recortes e não são promovidos a resultado do workspace.

Não foi executado `cargo` para produzir este relatório documental. Em
particular, **não se declara concluída a tríade final**:

```text
cargo test --workspace --all-features
cargo clippy --workspace --all-features -- -D warnings
cargo fmt --all
```

Ela deve ser executada sobre o diff consolidado, e o resultado deve ser ligado
ao commit/tag. Também não foram executados aqui testcontainers, fuzzing, Kani,
Miri, mutation, DAST, scanners de dependência, package/publish ou homologações
externas. Arquivos YAML que descrevem esses jobs não constituem resultado.

## Lacunas técnicas remanescentes

Em ordem de risco/impacto, o código e a governança ainda precisam de:

1. backend compartilhado e atômico para idempotência de webhooks/tracking e
   rate limiting distribuído, com testes multi-instância;
2. suíte normativa/biblioteca auditada para WebAuthn, além dos negativos atuais;
3. matriz de geração cobrindo todos os comandos/blueprints com fmt/check/smoke;
4. conclusão da arquitetura da Fase 3: Core sem dependência de ORM, uma pilha
   Security canônica e contratos/reexports coerentes no umbrella;
5. inventário semver e migração uniforme de constructors/builders fallible;
6. revisão específica da fronteira FFI/ABI do hot reload dev-only, incluindo
   testes apropriados de ciclo de vida; retenção de dylibs corrige o unload
   inseguro, mas não transforma ABI Rust dinâmica em contrato estável;
7. pipeline de tag que agregue SBOM, audit, compliance, digest e artefatos
   assinados, seguido da tríade final no commit exato;
8. alinhamento de [`WORKFLOWS.md`](../../WORKFLOWS.md) com 40 fuzz targets e com
   o caráter informativo de Kani/Miri/mutation;
9. somente quando houver mantenedor, ambiente de interoperabilidade e suite de
   contrato: NFS-e homologada, Alipay RSA2, storage remoto, replicação específica,
   messaging, MQTT, HSM/PQC e ciclo real de flash/boot. Até lá, manter
   `Unsupported`/experimental é o comportamento seguro.

