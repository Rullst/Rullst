# Deploy on Oracle Cloud

Oracle Cloud Infrastructure (OCI) offers an Always Free tier that includes
small virtual machines. The Always Free Ampere machines use **Arm** processors
(`aarch64`), so a Rullst image for them must be built for `linux/arm64`. This
guide covers the OCI-specific steps. After you can log in, everything is the
same as on every provider and lives in
[Deploy to a Linux VM](vm-deployment.md): read it alongside this page.

Official documentation: <https://docs.oracle.com/en-us/iaas/Content/home.htm>.
Free-tier shapes, quantities and availability change and differ by region and
account: check Oracle's current Always Free documentation before you rely on
them. Free Arm capacity in a popular region is sometimes unavailable; try
again later or pick another availability domain.

## Arm or x86?

- **Ampere A1 (Arm).** Build for `linux/arm64`. Rust and the generated
  `Dockerfile` work natively on Arm, so the simplest way is to **build on the
  VM itself** with `docker compose up -d --build` (step 4A of the shared guide).
  Ampere VMs are usually fast enough for this.
- **Building elsewhere.** On an x86-64 computer, build for Arm with
  `docker buildx build --platform linux/arm64 -t my-app:<tag> --load .`. This
  needs QEMU emulation (Docker Desktop includes it; on Linux install
  `qemu-user-static`) and an optimized Rust build under emulation is slow. For
  anything beyond a trial, build on the VM or on an Arm CI runner.
- **x86 micro shape.** Oracle's x86 free VM is very small (see its current
  memory); it cannot comfortably compile Rust. Build the image elsewhere with
  `--platform linux/amd64` and move it (step 4B of the shared guide).

## 1. Create the instance

In the OCI Console, **Compute**, **Instances**, **Create instance**:

1. **Image:** Ubuntu LTS (or Oracle Linux; this guide assumes Debian-family
   commands).
2. **Shape:** choose **Ampere** (`VM.Standard.A1.Flex`) and set the OCPUs and
   memory within the Always Free limits shown in the console.
3. **Networking:** use the default VCN and a **public subnet** with a public
   IPv4 address.
4. **SSH keys:** paste your public key. The default login user is `ubuntu` (or
   `opc` on Oracle Linux).

## 2. Firewall: two places

OCI blocks traffic in the cloud network **and** the image may block it on the
VM. You must open both.

**In the cloud network:** open the VCN's **Security Lists** (or the instance's
Network Security Group) and add ingress rules (source `0.0.0.0/0`, TCP) for
destination ports 22, 80 and 443 (and UDP 443 if you want HTTP/3; without it
browsers use HTTPS over TCP).

**On the VM:** Oracle's Ubuntu images ship with `iptables` rules that reject
other ports. Allow 80 and 443 before the final reject rule and save them:

```bash
sudo iptables -I INPUT 6 -m state --state NEW -p tcp --dport 80 -j ACCEPT
sudo iptables -I INPUT 6 -m state --state NEW -p tcp --dport 443 -j ACCEPT
sudo apt install -y iptables-persistent && sudo netfilter-persistent save
```

Check the rule numbers with `sudo iptables -L INPUT --line-numbers` first: the
new rules must come before the `REJECT` rule. On Oracle Linux use `firewalld`
(`sudo firewall-cmd --permanent --add-service={http,https}` and `--reload`).
Docker adds its own rules for published ports, so the cloud rules from the
first part are the ones that decide what the internet can reach.

## 3. DNS

Create an `A` record pointing to the instance's public IP address. For a stable
address, reserve a public IP instead of the ephemeral one, so it does not
change if you recreate the instance. Check with `dig +short your-domain.example`.

## 4. Log in and continue

```bash
ssh ubuntu@<ip-address>
```

Then follow [Deploy to a Linux VM](vm-deployment.md) from
[step 3](vm-deployment.md#3-install-docker-or-podman-on-the-vm). If you move
an image from another computer, make sure it was built for `linux/arm64`
(`docker image inspect my-app:<tag> --format '{{.Architecture}}'` should print
`arm64`).

## Oracle Cloud notes

- **Logs** stay on the instance (see [Logs](vm-deployment.md#7-logs)).
- Free accounts and idle instances can be reclaimed under Oracle's current
  policy; read the Always Free terms and keep your own off-VM
  [backups](vm-deployment.md#8-back-up-the-database-volume).
- Boot-volume backups are available in the console; they are not database
  backups.
