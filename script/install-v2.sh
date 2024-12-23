#Prepare base env
sudo apt update
sudo apt upgrade -y
sudo apt install gcc -y
sudo apt install libfontconfig -y
sudo apt install libfontconfig1-dev -y
sudo apt install dos2unix -y
sudo iptables -A INPUT -p tcp --dport 8080 -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 80 -j ACCEPT
sudo apt install unzip -y
sudo apt install git -y
#sudo apt install bind9 -y
#echo "net.ipv4.tcp_fastopen = 3" >> /etc/sysctl.conf
#sysctl -p
sudo curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustup update
#Create swap file
sudo swapoff /swapfile
sudo fallocate -l 2G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
sudo free -h
echo '/swapfile none swap sw 0 0' | sudo tee -a /etc/fstab

# Start install ppaass
# sudo ps -ef | grep ppaass-v2-proxy | grep -v grep | awk '{print $2}' | xargs sudo kill

sudo rm -rf /ppaass-v2/build
sudo rm -rf /ppaass-v2/sourcecode
# Build
sudo mkdir /ppaass-v2
sudo mkdir /ppaass-v2/sourcecode
sudo mkdir /ppaass-v2/build
sudo mkdir /ppaass-v2/build/resources
sudo mkdir /ppaass-v2/build/resources/proxy
sudo mkdir /ppaass-v2/build/resources/proxy/rsa
sudo mkdir /ppaass-v2/build/resources/proxy/forward_rsa
# Pull ppaass
cd /ppaass-v2/sourcecode
sudo git clone -b main https://github.com/quhxuxm/ppaass-v2.git ppaass-v2
sudo chmod 777 ppaass-v2
cd /ppaass-v2/sourcecode/ppaass-v2
sudo git pull

cd core
cargo build --release --package proxy

# ps -ef | grep gradle | grep -v grep | awk '{print $2}' | xargs kill -9
sudo cp -r /ppaass-v2/sourcecode/ppaass-v2/resources/proxy/* /ppaass-v2/build/resources
sudo cp -r /ppaass-v2/sourcecode/ppaass-v2/resources/proxy/rsa/* /ppaass-v2/build/resources/proxy/rsa
sudo cp -r /ppaass-v2/sourcecode/ppaass-v2/resources/proxy/forward_rsa/* /ppaass-v2/build/resources/proxy/forward_rsa
sudo cp /ppaass-v2/sourcecode/ppaass-v2/core/target/release/ppaass-proxy /ppaass-v2/build/ppaass-v2-proxy
sudo cp /ppaass-v2/sourcecode/ppaass-v2/script/start-proxy.sh /ppaass-v2/build/

sudo chmod 777 /ppaass-v2/build
cd /ppaass-v2/build
ls -l

sudo chmod 777 ppaass-v2-proxy
sudo chmod 777 *.sh
sudo dos2unix ./start-proxy.sh

#Start with the low configuration by default
sudo nohup ./start-proxy.sh >run.log 2>&1 &

ulimit -n 409600

