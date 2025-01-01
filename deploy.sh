(cd website && yarn install && yarn build)
ssh root@49.12.105.245 -t 'rm -rf /var/www/anagramdr'
scp -r website/build root@49.12.105.245:/var/www/anagramdr

(cd engine && tar -cvzf anagramdr.tar.gz Cargo.lock Cargo.toml data src)
scp engine/anagramdr.tar.gz root@49.12.105.245:/home/www-data
ssh root@49.12.105.245 -t 'rm -rf /home/www-data/anagramdr/*;tar -xf /home/www-data/anagramdr.tar.gz -C /home/www-data/anagramdr'
ssh root@49.12.105.245 -t 'cd /home/www-data/anagramdr; /root/.cargo/bin/cargo build --release; sudo systemctl restart anagramdr'