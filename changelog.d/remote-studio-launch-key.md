### Security

- The standalone `rullst-studio` (remote projects) answered every local
  account of the machine, because it checked only for a loopback peer: another
  user could open it on `127.0.0.1:5556`, read the monitoring documents it
  fetches with the decrypted tokens and change its projects and settings. Each
  start now prints `http://127.0.0.1:<port>/studio?key=<key>` with a random
  256-bit key; opening it sets an `HttpOnly`, `SameSite=Strict` session cookie,
  and every request without that cookie gets `403`. Open the printed address
  after each start; anyone who can read it can use Studio until it restarts.
