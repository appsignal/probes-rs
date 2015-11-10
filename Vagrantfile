# -*- mode: ruby -*-
# vi: set ft=ruby :

Vagrant.configure(2) do |config|
  config.vm.box = 'http://files.vagrantup.com/lucid64.box'

  config.vm.provision 'shell', inline: <<-SHELL
    sudo apt-get update
    sudo apt-get upgrade -y
    sudo apt-get install build-essential curl git-core -y
    curl -o blastoff.sh https://raw.githubusercontent.com/brson/multirust/master/blastoff.sh
    sh blastoff.sh --yes
    rm -f blastoff.sh
    multirust default stable
  SHELL
end
