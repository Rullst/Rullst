import base64
from cryptography.hazmat.primitives.asymmetric import rsa
from cryptography.hazmat.primitives import serialization

# Generate private key
private_key = rsa.generate_private_key(
    public_exponent=65537,
    key_size=2048,
)

# Private key in PEM
pem = private_key.private_bytes(
    encoding=serialization.Encoding.PEM,
    format=serialization.PrivateFormat.PKCS8,
    encryption_algorithm=serialization.NoEncryption()
).decode('utf-8')

# Public key components
public_key = private_key.public_key()
numbers = public_key.public_numbers()
n = base64.urlsafe_b64encode(numbers.n.to_bytes((numbers.n.bit_length() + 7) // 8, byteorder='big')).rstrip(b'=').decode('ascii')
e = base64.urlsafe_b64encode(numbers.e.to_bytes((numbers.e.bit_length() + 7) // 8, byteorder='big')).rstrip(b'=').decode('ascii')

print("=== PEM ===")
print(pem)
print("=== N ===")
print(n)
print("=== E ===")
print(e)
