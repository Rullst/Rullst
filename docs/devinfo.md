Info for portuguese developers

- Trabalhando localmente no framework: 
cargo install --path cargo-rullst (na pasta do repo)


- Testando a versão como um usuário final: 
cargo install cargo-rullst --force


Comando para verificar atualizações no projeto:
cargo outdated --root-deps-only


cargo clean ; cargo test --all-features ; cargo clippy --all-features --fix

cargo fmt ; cargo publish --dry-run



Lançamento da nova versão:

git tag -a v7.0.2 -m "Release v7.0.2"

git push origin v7.0.2