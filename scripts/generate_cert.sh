
if [[ ! -d "$1" ]]; then
    mkdir $1
fi

# Generate root private key
openssl genrsa -out "${1:-./}/rootCA.key" 2048

# Generate self-signed root certificate
openssl req -x509 -new -nodes -key "${1:-./}/rootCA.key" -sha256 -days 365 -out "${1:-./}/rootCA.crt" \
    -subj "/C=US/ST=Nordjylland/L=Aalborg/O=SAM/OU=IT/CN=localhost"

# Generate server private key
openssl genrsa -out "${1:-./}/server.key" 2048

# Generate Certificate Signing Request (CSR)
openssl req -new -key "${1:-./}/server.key" -out "${1:-./}/server.csr" -config server_cert_ext.cnf

# Sign the server certificate using root certificate
openssl x509 -req -in "${1:-./}/server.csr" -CA "${1:-./}/rootCA.crt" -CAkey "${1:-./}/rootCA.key" -CAcreateserial \
    -out "${1:-./}/server.crt" -days 365 -sha256 -extfile server_cert_ext.cnf -extensions v3_req

# Generate server private key
openssl genrsa -out "${1:-./}/proxy.key" 2048

# Generate Certificate Signing Request (CSR)
openssl req -new -key "${1:-./}/proxy.key" -out "${1:-./}/proxy.csr" -config server_cert_ext.cnf

# Sign the server certificate using root certificate
openssl x509 -req -in "${1:-./}/proxy.csr" -CA "${1:-./}/rootCA.crt" -CAkey "${1:-./}/rootCA.key" -CAcreateserial \
    -out "${1:-./}/proxy.crt" -days 365 -sha256 -extfile server_cert_ext.cnf -extensions v3_req

# Generate server private key
openssl genrsa -out "${1:-./}/client.key" 2048

# Generate client key and certificate signing request (CSR)
openssl req -new -key "${1:-./}/client.key" -out "${1:-./}/client.csr" -config server_cert_ext.cnf

# Sign the client certificate with the CA
openssl x509 -req -in "${1:-./}/client.csr" -CA "${1:-./}/rootCA.crt" -CAkey "${1:-./}/rootCA.key"  -CAcreateserial \
    -out "${1:-./}/client.crt" -days 365 -sha256 -extfile server_cert_ext.cnf -extensions v3_req

rm "${1:-./}/rootCA.srl" "${1:-./}/rootCA.key" "${1:-./}/server.csr" "${1:-./}/client.csr" "${1:-./}/proxy.csr"