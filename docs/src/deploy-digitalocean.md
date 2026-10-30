# Deploy on DigitalOcean

DigitalOcean calls its virtual machines "Droplets". This guide covers only the
DigitalOcean-specific steps. After you can log in, everything is the same on
every provider and lives in [Deploy to a Linux VM](vm-deployment.md): read it
alongside this page.

Official documentation: <https://docs.digitalocean.com/products/droplets/>.
Check the provider's current pricing, regions and limits; they change.

## 1. Create the Droplet

In the [control panel](https://cloud.digitalocean.com/), choose **Create**,
then **Droplets**:

1. **Region:** closest to your users.
2. **Image:** Debian or Ubuntu LTS.
3. **Size:** a Basic (shared CPU) Droplet with 2 GB of memory or more is
   comfortable for one small app. The smallest sizes can run the app but may
   run out of memory while building the image; see the swap note in the
   [shared guide](vm-deployment.md#2-create-the-vm-and-the-dns-record), or
   build the image elsewhere.
4. **Authentication:** choose **SSH Key** and add your public key.
5. Optional **Backups** are a paid whole-Droplet feature; they do not replace
   [database backups](vm-deployment.md#8-back-up-the-database-volume).

Note the Droplet's public IPv4 address.

## 2. Firewall: 22, 80 and 443 only

Under **Networking**, **Firewalls**, create a Cloud Firewall with these inbound
rules, leave outbound rules at their defaults, and apply it to the Droplet:

| Type | Protocol | Port | Sources |
| :--- | :--- | :--- | :--- |
| SSH | TCP | 22 | your IP address if stable, otherwise all |
| HTTP | TCP | 80 | all IPv4 (and IPv6 if you use it) |
| HTTPS | TCP | 443 | all IPv4 (and IPv6 if you use it) |

Port 80 must stay open: Caddy uses it to obtain and renew certificates. Add a
UDP 443 rule as well if you want HTTP/3; without it browsers use HTTPS over
TCP.

If you also run a host firewall such as `ufw`, allow the same three ports
there. Note that Docker publishes container ports by changing the firewall
rules itself, which can bypass `ufw`; the cloud firewall above is the reliable
one.

## 3. DNS

Add the domain under **Networking**, **Domains** (or use any other DNS host),
and create an `A` record pointing to the Droplet's IP address. Check with
`dig +short your-domain.example`.

## 4. Log in and continue

```bash
ssh root@<ip-address>
```

Create a normal user with `sudo` rights (`adduser deploy && usermod -aG sudo
deploy`, then copy `~/.ssh/authorized_keys` to its `.ssh` directory) and use it
from now on. Then follow [Deploy to a Linux VM](vm-deployment.md) from
[step 3](vm-deployment.md#3-install-docker-or-podman-on-the-vm).

## DigitalOcean notes

- **Logs** stay on the Droplet (see [Logs](vm-deployment.md#7-logs)).
- **Snapshots** are available before risky changes and are not database
  backups.
- A **Recovery Console** in the control panel helps if SSH stops working.
- Managed databases are an alternative to a database on the Droplet: use the
  connection URL as `DATABASE_URL` and allow only your Droplet to connect.
