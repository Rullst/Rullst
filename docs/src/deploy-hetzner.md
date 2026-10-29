# Deploy on Hetzner Cloud

Hetzner Cloud sells simple virtual machines ("Cloud Servers") in European and
some other regions. This guide covers only the Hetzner-specific steps. Everything
after you can log in to the VM is the same on every provider and lives in
[Deploy to a Linux VM](vm-deployment.md): read it alongside this page.

Official documentation: <https://docs.hetzner.com/cloud/>. Check the provider's
current pricing, locations and limits before you choose; they change.

## 1. Create the server

In the [Cloud Console](https://console.hetzner.cloud/):

1. Create a project, then **Add Server**.
2. **Location:** pick the one closest to your users.
3. **Image:** Debian or Ubuntu LTS.
4. **Type:** a small shared-CPU type with 2 GB of memory or more is comfortable
   for one small app. Both x86-64 and Arm (Ampere) types exist; an Arm type
   needs a `linux/arm64` image (see
   [step 4 of the shared guide](vm-deployment.md#4-get-the-application-onto-the-vm)).
5. **SSH keys:** add your public key (`~/.ssh/id_ed25519.pub`). Do not choose a
   password login.
6. **Firewalls:** attach the firewall you create in the next section.
7. Backups are an optional paid add-on for the whole server; they do not
   replace [database backups](vm-deployment.md#8-back-up-the-database-volume).

Note the server's public IPv4 address (and IPv6, if you want it).

## 2. Firewall: 22, 80 and 443 only

Under **Firewalls**, create one with these inbound rules and apply it to the
server:

| Protocol | Port | Source |
| :--- | :--- | :--- |
| TCP | 22 | your own IP address if it is stable, otherwise any |
| TCP | 80 | any |
| TCP | 443 | any |

Hetzner firewalls are stateful, so replies are allowed automatically. Port 80
must stay open: Caddy uses it to obtain and renew certificates. Add UDP 443 as
well if you want HTTP/3; without it browsers use HTTPS over TCP.

## 3. DNS

Create an `A` record for your domain with the server's IPv4 address (and an
`AAAA` record for IPv6). You can use Hetzner DNS or any other DNS provider.
Check with `dig +short your-domain.example`.

## 4. Log in and continue

```bash
ssh root@<ip-address>
```

Create a normal user with `sudo` rights and use it from now on (root login is
convenient but risky):

```bash
adduser deploy && usermod -aG sudo deploy
mkdir -p /home/deploy/.ssh && cp ~/.ssh/authorized_keys /home/deploy/.ssh/ \
  && chown -R deploy:deploy /home/deploy/.ssh
```

Then follow [Deploy to a Linux VM](vm-deployment.md) from
[step 3](vm-deployment.md#3-install-docker-or-podman-on-the-vm): install Docker
or Podman, copy or build the app, set the
[production settings](vm-deployment.md#5-production-settings-that-matter),
start it with `compose up`, and set up
[backups](vm-deployment.md#8-back-up-the-database-volume),
[updates and rollback](vm-deployment.md#9-update-and-roll-back-with-image-tags).

## Hetzner notes

- **Logs** stay on the server (see [Logs](vm-deployment.md#7-logs)); the Cloud
  Console does not collect them.
- **Snapshots** of the server are available in the Console and are useful before
  a risky change. They are not a database backup.
- The Cloud Console's **Rescue** system and console access help if you lock
  yourself out of SSH.
