#!/bin/bash

# Create symlink, removing existing file/dir if it's not already a link
make_link() {
    local target="$1"
    local link_name="$2"

    if [ -L "$link_name" ]; then
        ln -sf "$target" "$link_name"
    elif [ -e "$link_name" ]; then
        rm -rf "$link_name"
        ln -s "$target" "$link_name"
    else
        ln -s "$target" "$link_name"
    fi
}

command_exists() {
    command -v "$1" &> /dev/null
}

echo ">>> Installing development packages..."

# Install apt packages
# Use round brackets for an array declaration
PACKAGES=(
    git
    zsh
    neovim
    tmux
    fzf
    gh
    ripgrep
    bat
    lsd
    fastfetch
    unzip
    curl
    wget
    gcc
    make
    cmake
    git-delta
    dotnet-sdk-10.0
    libssl-dev 
    libpq-dev 
    build-essential
)

sudo add-apt-repository ppa:zhangsongcui3371/fastfetch -y

echo "apt update..."
sudo apt update

echo "Installing packages: ${PACKAGES[*]}..."
sudo apt install "${PACKAGES[@]}" -y

# Get dotfiles
cd ~
git clone https://github.com/markzuber/dotfiles

# -----------------------------------------------------------------------------
# Oh-My-Zsh and plugins
# -----------------------------------------------------------------------------

echo ">>> Installing Oh-My-Zsh..."
if [ ! -d "$HOME/.oh-my-zsh" ]; then
    RUNZSH=no sh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)"
else
    echo ">>> Oh-My-Zsh already installed"
fi

ZSH_CUSTOM="${ZSH_CUSTOM:-$HOME/.oh-my-zsh/custom}"

echo ">>> Installing zsh plugins..."
if [ ! -d "$ZSH_CUSTOM/plugins/zsh-autosuggestions" ]; then
    git clone https://github.com/zsh-users/zsh-autosuggestions "$ZSH_CUSTOM/plugins/zsh-autosuggestions"
else
    echo ">>> zsh-autosuggestions already installed"
fi

if [ ! -d "$ZSH_CUSTOM/plugins/zsh-syntax-highlighting" ]; then
    git clone https://github.com/zsh-users/zsh-syntax-highlighting.git "$ZSH_CUSTOM/plugins/zsh-syntax-highlighting"
else
    echo ">>> zsh-syntax-highlighting already installed"
fi

if [ ! -d "$ZSH_CUSTOM/plugins/fzf-tab" ]; then
    git clone https://github.com/Aloxaf/fzf-tab "$ZSH_CUSTOM/plugins/fzf-tab"
else
    echo ">>> fzf-tab already installed"
fi

echo ">>> Installing Powerlevel10k..."
if [ ! -d "$ZSH_CUSTOM/themes/powerlevel10k" ]; then
    git clone --depth=1 https://github.com/romkatv/powerlevel10k.git "$ZSH_CUSTOM/themes/powerlevel10k"
else
    echo ">>> Powerlevel10k already installed"
fi

# -----------------------------------------------------------------------------
# Install git credential manager
# thanks microsoft for not just making this a package you can install via apt install
# -----------------------------------------------------------------------------

pushd /tmp
wget https://github.com/git-ecosystem/git-credential-manager/releases/download/v2.7.0/gcm-linux-x64-2.7.0.deb
sudo dpkg -i gcm-linux-x64-2.7.0.deb
popd

# -----------------------------------------------------------------------------
# install UV
# -----------------------------------------------------------------------------

curl -LsSf https://astral.sh/uv/install.sh | sh

# -----------------------------------------------------------------------------
# Install Node.js via nvm
# -----------------------------------------------------------------------------

echo ">>> Installing nvm and Node.js..."
if [ ! -d "$HOME/.nvm" ]; then
    curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.3/install.sh | bash
else
    echo ">>> nvm already installed"
fi

# Source nvm and install Node.js
export NVM_DIR="$HOME/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"

if ! command_exists node; then
    echo ">>> Installing Node.js LTS..."
    nvm install --lts
    nvm use --lts
else
    echo ">>> Node.js already installed: $(node --version)"
fi

# Install global npm packages
echo ">>> Installing global npm packages..."
NPM_GLOBALS=(typescript ts-node prettier eslint)
for pkg in "${NPM_GLOBALS[@]}"; do
    if ! npm list -g "$pkg" &> /dev/null; then
        npm install -g "$pkg"
    else
        echo ">>> npm package $pkg already installed"
    fi
done

# -----------------------------------------------------------------------------
# Symlink dotfiles
# -----------------------------------------------------------------------------

echo ">>> Linking dotfiles..."

# bat/cat
make_link /usr/bin/batcat ~/.local/bin/bat

# zshrc
make_link ~/dotfiles/zsh/.zshrc ~/.zshrc

# ghostty config
mkdir -p ~/.config/ghostty
make_link ~/dotfiles/ghostty/.config/ghostty/config ~/.config/ghostty/config

# p10k
make_link ~/dotfiles/p10k/.p10k.zsh ~/.p10k.zsh

# nvim
make_link ~/dotfiles/nvim/.config/nvim ~/.config/nvim

# tmux
make_link ~/dotfiles/tmux/.tmux.conf ~/.tmux.conf

# editorconfig
make_link ~/dotfiles/editorconfig/.editorconfig ~/.editorconfig

# gitconfig
make_link ~/dotfiles/git/.gitconfig ~/.gitconfig

# -----------------------------------------------------------------------------
# Install Rust via rustup
# -----------------------------------------------------------------------------

echo ">>> Installing Rust..."
if ! command_exists rustc; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    echo ">>> Rust already installed: $(rustc --version)"
fi

# -----------------------------------------------------------------------------
# Ensure cargo env is sourced
# -----------------------------------------------------------------------------

[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"

# Install claude
curl -fsSL https://claude.ai/install.sh | bash