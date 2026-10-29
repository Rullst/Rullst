# Deploy on Google Cloud (Compute Engine)

This guide runs your app on a Compute Engine virtual machine. Google Cloud's
free tier has included a small `e2-micro` instance in selected US regions; the
terms, regions and limits change, so check Google's current "Free Tier" page
before you rely on it. An `e2-micro` has very little memory, so **do not
compile Rust on it**: build the image on your own computer or in CI, and only
run it on the VM (step 4B of the shared guide, or a registry). After you can
log in, everything is the same as on every provider and lives in
[Deploy to a Linux VM](vm-deployment.md): read it alongside this page.

Official documentation: <https://cloud.google.com/compute/docs>.

## 1. Create the VM

In the [Google Cloud console](https://console.cloud.google.com/) (or with the
`gcloud` CLI), enable the Compute Engine API for a project, then **Compute
Engine**, **VM instances**, **Create instance**:

1. **Region and zone:** for the free tier, one of the eligible regions listed
   in Google's current documentation; otherwise the closest to your users.
2. **Machine type:** `e2-micro` for a trial. For a real app, a larger type
   (for example `e2-small` or `e2-medium`) is much more comfortable: check the
   current pricing.
3. **Boot disk:** Debian or Ubuntu LTS. Keep the disk type and size within the
   free-tier limits if you want to stay free.
4. **Firewall:** tick **Allow HTTP traffic** and **Allow HTTPS traffic** (see
   the next section; SSH is allowed by the default rules).
5. **Networking:** under network interfaces, use a **static external IP
   address** (reserve one) so the DNS record does not break after a restart.
   Check the current price of reserved addresses.

## 2. Firewall: 22, 80 and 443 only

Compute Engine firewall rules apply to the whole VPC network. The default
network already allows SSH (22). The HTTP and HTTPS checkboxes create rules for
instances with the network tags `http-server` and `https-server`. To control
this yourself:

```bash
gcloud compute firewall-rules create allow-web \
  --network default --direction INGRESS --allow tcp:80,tcp:443 \
  --target-tags web --source-ranges 0.0.0.0/0
gcloud compute instances add-tags <instance-name> --tags web --zone <zone>
```

Also review the default network's other rules (for example the one that allows
all internal traffic or RDP) and remove what you do not use. Port 80 must stay
open: Caddy uses it to obtain and renew certificates. For HTTP/3 also allow
`udp:443`; without it browsers use HTTPS over TCP.

## 3. DNS

Create an `A` record pointing to the reserved external IP address (in Cloud DNS
or any DNS host). Check with `dig +short your-domain.example`.

## 4. Log in and continue

```bash
gcloud compute ssh <instance-name> --zone <zone>
```

or use the **SSH** button in the console. Then follow
[Deploy to a Linux VM](vm-deployment.md) from
[step 3](vm-deployment.md#3-install-docker-or-podman-on-the-vm). On a small
machine, build elsewhere and transfer the image. An alternative to
`docker save`/`load` is Google's Artifact Registry
(<https://cloud.google.com/artifact-registry/docs>): push the tagged image
there, and pull it on the VM.

## Google Cloud notes

- **Logs** from your containers stay on the VM (see
  [Logs](vm-deployment.md#7-logs)). The Ops Agent can ship host logs to Cloud
  Logging if you want that; it is optional and may cost money.
- **Snapshots** of the persistent disk are available, including schedules. They
  are not database backups: keep a
  [database backup](vm-deployment.md#8-back-up-the-database-volume) too.
- Compute Engine can use a managed database (Cloud SQL) instead of a database
  on the VM; use its connection URL as `DATABASE_URL`.
- Cloud Run and GKE are different services with their own guides; for a
  Kubernetes outline see [Advanced deployment](advanced-deployment.md).
