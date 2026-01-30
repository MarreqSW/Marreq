#!/bin/bash
set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "${SCRIPT_DIR}")"

echo -e "${BLUE}"
echo "╔═════════════════════════════════════════════════════════════╗"
echo "║                                                             ║"
echo "║   ██████╗ ███████╗ ██████╗ ███╗   ███╗ █████╗ ███╗   ██╗    ║"
echo "║   ██╔══██╗██╔════╝██╔═══██╗████╗ ████║██╔══██╗████╗  ██║    ║"
echo "║   ██████╔╝█████╗  ██║   ██║██╔████╔██║███████║██╔██╗ ██║    ║"
echo "║   ██╔══██╗██╔══╝  ██║▄▄ ██║██║╚██╔╝██║██╔══██║██║╚██╗██║    ║"
echo "║   ██║  ██║███████╗╚██████╔╝██║ ╚═╝ ██║██║  ██║██║ ╚████║    ║"
echo "║   ╚═╝  ╚═╝╚══════╝ ╚══▀▀═╝ ╚═╝     ╚═╝╚═╝  ╚═╝╚═╝  ╚═══╝    ║"
echo "║                                                             ║"
echo "║        🚀 Lazy Setup Script - One-Click Install 🚀          ║"
echo "║                                                             ║"
echo "╚═════════════════════════════════════════════════════════════╝"
echo -e "${NC}"

# Configuration
INSTALL_OLLAMA=${INSTALL_OLLAMA:-true}
OLLAMA_EMBEDDING_MODEL=${OLLAMA_EMBEDDING_MODEL:-"nomic-embed-text"}
OLLAMA_RAG_MODEL=${OLLAMA_RAG_MODEL:-"llama3.2"}
SKIP_BUILD=${SKIP_BUILD:-false}
AUTO_START=${AUTO_START:-false}

# Track what was installed
INSTALLED_ITEMS=()

print_step() {
    echo ""
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}📦 $1${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

# Detect package manager
detect_package_manager() {
    if command -v apt-get &> /dev/null; then
        echo "apt"
    elif command -v dnf &> /dev/null; then
        echo "dnf"
    elif command -v yum &> /dev/null; then
        echo "yum"
    elif command -v pacman &> /dev/null; then
        echo "pacman"
    elif command -v zypper &> /dev/null; then
        echo "zypper"
    elif command -v brew &> /dev/null; then
        echo "brew"
    else
        echo "unknown"
    fi
}

# Install packages based on package manager
install_package() {
    local pkg_apt=$1
    local pkg_dnf=${2:-$1}
    local pkg_pacman=${3:-$1}
    local pkg_brew=${4:-$1}
    
    local pm=$(detect_package_manager)
    
    case $pm in
        apt)
            sudo apt-get install -y "$pkg_apt"
            ;;
        dnf)
            sudo dnf install -y "$pkg_dnf"
            ;;
        yum)
            sudo yum install -y "$pkg_dnf"
            ;;
        pacman)
            sudo pacman -S --noconfirm "$pkg_pacman"
            ;;
        zypper)
            sudo zypper install -y "$pkg_apt"
            ;;
        brew)
            brew install "$pkg_brew"
            ;;
        *)
            print_error "Unknown package manager. Please install $pkg_apt manually."
            return 1
            ;;
    esac
}

# Update package manager cache
update_package_cache() {
    local pm=$(detect_package_manager)
    
    case $pm in
        apt)
            sudo apt-get update
            ;;
        dnf|yum)
            # dnf/yum updates cache automatically
            ;;
        pacman)
            sudo pacman -Sy
            ;;
        zypper)
            sudo zypper refresh
            ;;
        brew)
            brew update
            ;;
    esac
}

# ============================================================================
# STEP 1: Check and install system dependencies
# ============================================================================

print_step "Step 1/7: Checking system dependencies..."

# Check for curl
if ! command -v curl &> /dev/null; then
    print_warning "curl not found. Installing..."
    update_package_cache
    install_package curl curl curl curl
    INSTALLED_ITEMS+=("curl")
else
    print_success "curl is installed"
fi

# Check for git
if ! command -v git &> /dev/null; then
    print_warning "git not found. Installing..."
    install_package git git git git
    INSTALLED_ITEMS+=("git")
else
    print_success "git is installed"
fi

# ============================================================================
# STEP 2: Check and install Docker
# ============================================================================

print_step "Step 2/7: Checking Docker installation..."

if ! command -v docker &> /dev/null; then
    print_warning "Docker not found. Installing..."
    
    if [[ "$(uname)" == "Linux" ]]; then
        # Install Docker on Linux
        curl -fsSL https://get.docker.com | sh
        sudo usermod -aG docker $USER
        INSTALLED_ITEMS+=("Docker")
        print_warning "You may need to log out and back in for Docker group membership to take effect."
    elif [[ "$(uname)" == "Darwin" ]]; then
        print_error "Please install Docker Desktop from https://www.docker.com/products/docker-desktop"
        exit 1
    else
        print_error "Please install Docker manually from https://docs.docker.com/get-docker/"
        exit 1
    fi
else
    print_success "Docker is installed"
fi

# Check if Docker daemon is running
if ! docker info &> /dev/null; then
    print_warning "Docker daemon is not running. Attempting to start..."
    if command -v systemctl &> /dev/null; then
        sudo systemctl start docker
        sleep 2
    else
        print_error "Please start Docker daemon manually and re-run this script."
        exit 1
    fi
fi
print_success "Docker daemon is running"

# Check for Docker Compose
if docker compose version &> /dev/null; then
    DC="docker compose"
    print_success "Docker Compose (v2) is available"
elif docker-compose version &> /dev/null; then
    DC="docker-compose"
    print_success "Docker Compose (v1) is available"
else
    print_error "Docker Compose not found. Please install it."
    exit 1
fi

# ============================================================================
# STEP 3: Check and install Rust
# ============================================================================

print_step "Step 3/7: Checking Rust installation..."

if ! command -v rustc &> /dev/null; then
    print_warning "Rust not found. Installing via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    INSTALLED_ITEMS+=("Rust")
else
    print_success "Rust is installed ($(rustc --version))"
fi

# Ensure cargo is in PATH
if ! command -v cargo &> /dev/null; then
    source "$HOME/.cargo/env" 2>/dev/null || true
fi

# ============================================================================
# STEP 4: Install build dependencies (clang, libclang)
# ============================================================================

print_step "Step 4/7: Checking build dependencies..."

# Check for clang/libclang (required by xlsxwriter)
NEED_CLANG=false
if ! command -v clang &> /dev/null; then
    NEED_CLANG=true
fi

# Check for libclang
if [[ "$(detect_package_manager)" == "apt" ]]; then
    if ! dpkg -l | grep -q libclang-dev; then
        NEED_CLANG=true
    fi
fi

if [[ "$NEED_CLANG" == "true" ]]; then
    print_warning "Installing clang and libclang-dev (required for xlsxwriter)..."
    update_package_cache
    
    pm=$(detect_package_manager)
    case $pm in
        apt)
            sudo apt-get install -y clang libclang-dev
            ;;
        dnf|yum)
            sudo dnf install -y clang clang-devel
            ;;
        pacman)
            sudo pacman -S --noconfirm clang
            ;;
        brew)
            brew install llvm
            ;;
        *)
            print_warning "Please install clang and libclang-dev manually"
            ;;
    esac
    INSTALLED_ITEMS+=("clang/libclang-dev")
else
    print_success "clang and libclang are installed"
fi

# Check for pkg-config
if ! command -v pkg-config &> /dev/null; then
    print_warning "Installing pkg-config..."
    install_package pkg-config pkg-config pkg-config pkg-config
    INSTALLED_ITEMS+=("pkg-config")
else
    print_success "pkg-config is installed"
fi

# Check for OpenSSL development libraries
if [[ "$(detect_package_manager)" == "apt" ]]; then
    if ! dpkg -l | grep -q libssl-dev; then
        print_warning "Installing libssl-dev..."
        sudo apt-get install -y libssl-dev
        INSTALLED_ITEMS+=("libssl-dev")
    else
        print_success "libssl-dev is installed"
    fi
elif [[ "$(detect_package_manager)" == "dnf" ]] || [[ "$(detect_package_manager)" == "yum" ]]; then
    if ! rpm -q openssl-devel &> /dev/null; then
        print_warning "Installing openssl-devel..."
        sudo dnf install -y openssl-devel
        INSTALLED_ITEMS+=("openssl-devel")
    else
        print_success "openssl-devel is installed"
    fi
fi

# ============================================================================
# STEP 5: Install and configure Ollama
# ============================================================================

print_step "Step 5/7: Setting up Ollama for semantic search..."

if [[ "$INSTALL_OLLAMA" == "true" ]]; then
    if ! command -v ollama &> /dev/null; then
        print_info "Installing Ollama..."
        
        if [[ "$(uname)" == "Linux" ]]; then
            curl -fsSL https://ollama.ai/install.sh | sh
            INSTALLED_ITEMS+=("Ollama")
        elif [[ "$(uname)" == "Darwin" ]]; then
            if command -v brew &> /dev/null; then
                brew install ollama
                INSTALLED_ITEMS+=("Ollama")
            else
                print_error "Please install Ollama from https://ollama.ai/download"
                exit 1
            fi
        else
            print_warning "Please install Ollama manually from https://ollama.ai/download"
        fi
    else
        print_success "Ollama is already installed"
    fi
    
    # Start Ollama service if not running
    if ! curl -s http://localhost:11434/api/tags &> /dev/null; then
        print_info "Starting Ollama service..."
        
        # Try to start via systemd first
        if command -v systemctl &> /dev/null && systemctl list-unit-files | grep -q ollama; then
            sudo systemctl start ollama
            sleep 3
        else
            # Start in background
            nohup ollama serve > /tmp/ollama.log 2>&1 &
            sleep 5
        fi
        
        # Wait for Ollama to be ready
        MAX_RETRIES=30
        RETRY_COUNT=0
        until curl -s http://localhost:11434/api/tags &> /dev/null; do
            RETRY_COUNT=$((RETRY_COUNT + 1))
            if [[ ${RETRY_COUNT} -ge ${MAX_RETRIES} ]]; then
                print_warning "Ollama didn't start. You can start it manually with: ollama serve"
                break
            fi
            echo "   Waiting for Ollama... (attempt ${RETRY_COUNT}/${MAX_RETRIES})"
            sleep 2
        done
    fi
    
    # Check if Ollama is running now
    if curl -s http://localhost:11434/api/tags &> /dev/null; then
        print_success "Ollama is running"
        
        # Pull embedding model
        print_info "Pulling embedding model: ${OLLAMA_EMBEDDING_MODEL}..."
        if ollama pull "${OLLAMA_EMBEDDING_MODEL}"; then
            print_success "Embedding model ready: ${OLLAMA_EMBEDDING_MODEL}"
        else
            print_warning "Failed to pull ${OLLAMA_EMBEDDING_MODEL}. You can try manually: ollama pull ${OLLAMA_EMBEDDING_MODEL}"
        fi
        
        # Pull RAG model (optional)
        print_info "Pulling RAG model: ${OLLAMA_RAG_MODEL}..."
        if ollama pull "${OLLAMA_RAG_MODEL}"; then
            print_success "RAG model ready: ${OLLAMA_RAG_MODEL}"
        else
            print_warning "Failed to pull ${OLLAMA_RAG_MODEL}. You can try manually: ollama pull ${OLLAMA_RAG_MODEL}"
        fi
    else
        print_warning "Ollama is not running. You can start it with: ollama serve"
    fi
else
    print_info "Skipping Ollama installation (INSTALL_OLLAMA=false)"
fi

# ============================================================================
# STEP 6: Set up the database
# ============================================================================

print_step "Step 6/7: Setting up PostgreSQL database..."

cd "${PROJECT_ROOT}"

# Start the database container
print_info "Starting database container..."
$DC up -d db

# Wait for database to be ready
print_info "Waiting for PostgreSQL to be ready..."
DB_CID=$($DC ps -q db || true)
MAX_RETRIES=30
RETRY_COUNT=0
until docker exec "${DB_CID}" pg_isready -U rust -q 2>/dev/null; do
    RETRY_COUNT=$((RETRY_COUNT + 1))
    if [[ ${RETRY_COUNT} -ge ${MAX_RETRIES} ]]; then
        print_error "PostgreSQL failed to become ready after ${MAX_RETRIES} attempts."
        exit 1
    fi
    echo "   Waiting for database... (attempt ${RETRY_COUNT}/${MAX_RETRIES})"
    sleep 2
done
print_success "PostgreSQL is ready"

# Run the database setup script
if [[ -f "${SCRIPT_DIR}/setup_database.sh" ]]; then
    print_info "Running database setup script..."
    bash "${SCRIPT_DIR}/setup_database.sh"
else
    print_warning "Database setup script not found. Running minimal setup..."
    
    # Create database if it doesn't exist
    if ! docker exec "${DB_CID}" psql -U rust -d postgres -tAc "SELECT 1 FROM pg_database WHERE datname='reqman'" | grep -q 1; then
        docker exec "${DB_CID}" psql -U rust -d postgres -c "CREATE DATABASE reqman;"
        print_success "Created 'reqman' database"
    fi
    
    # Run migrations if diesel is available
    if command -v diesel &> /dev/null; then
        diesel migration run
        print_success "Ran database migrations"
    fi
fi

print_success "Database setup complete"

# ============================================================================
# STEP 7: Build the application
# ============================================================================

print_step "Step 7/7: Building ReqMan..."

cd "${PROJECT_ROOT}"

if [[ "$SKIP_BUILD" == "false" ]]; then
    print_info "Compiling ReqMan (this may take a few minutes)..."
    
    if cargo build --release; then
        print_success "ReqMan compiled successfully"
    else
        print_error "Failed to compile ReqMan"
        exit 1
    fi
else
    print_info "Skipping build (SKIP_BUILD=true)"
fi

# ============================================================================
# Create .env file if it doesn't exist
# ============================================================================

if [[ ! -f "${PROJECT_ROOT}/.env" ]]; then
    print_info "Creating .env file with default configuration..."
    cat > "${PROJECT_ROOT}/.env" << EOF
# ReqMan Environment Configuration
# Generated by lazy_setup.sh on $(date)

# Database Configuration
DATABASE_URL=postgres://rust:rust@127.0.0.1:5432/reqman

# Semantic Search Configuration
EMBEDDINGS_ENABLED=true
EMBEDDING_PROVIDER=ollama
EMBEDDING_MODEL=${OLLAMA_EMBEDDING_MODEL}
OLLAMA_URL=http://localhost:11434

# RAG Configuration (optional)
RAG_ENABLED=true
RAG_MODEL=${OLLAMA_RAG_MODEL}
RAG_MAX_TOKENS=1024
RAG_TOP_K=10
EOF
    print_success "Created .env file"
fi

# ============================================================================
# Summary
# ============================================================================

echo ""
echo -e "${GREEN}"
echo "╔══════════════════════════════════════════════════════════════════╗"
echo "║                                                                  ║"
echo "║        🎉 ReqMan Setup Complete! 🎉                              ║"
echo "║                                                                  ║"
echo "╚══════════════════════════════════════════════════════════════════╝"
echo -e "${NC}"

if [[ ${#INSTALLED_ITEMS[@]} -gt 0 ]]; then
    echo -e "${BLUE}📦 Installed Components:${NC}"
    for item in "${INSTALLED_ITEMS[@]}"; do
        echo "   • $item"
    done
    echo ""
fi

echo -e "${BLUE}🔐 Login Credentials (all users have password: 'password'):${NC}"
echo "   • alice (Admin) - Alice Johnson"
echo "   • dr_smith (Admin) - Dr. Sarah Smith"
echo "   • eng_jones - Engineer Mike Jones"
echo "   • tech_lee - Technician Lisa Lee"
echo "   • qa_wilson - QA Specialist Tom Wilson"
echo "   • admin (Admin) - System Administrator"
echo ""

echo -e "${BLUE}🌐 Services:${NC}"
echo "   • ReqMan:    http://localhost:8000"
echo "   • Adminer:   http://localhost:8080 (Database UI)"
echo "   • Ollama:    http://localhost:11434"
echo ""

echo -e "${BLUE}🚀 To start ReqMan:${NC}"
echo "   cargo run --release --bin req_man"
echo ""
echo -e "${BLUE}   Or for development:${NC}"
echo "   cargo run --bin req_man"
echo ""

echo -e "${BLUE}📖 Useful Commands:${NC}"
echo "   • View logs:           $DC logs -f"
echo "   • Stop database:       $DC down"
echo "   • Reset database:      ./scripts/clear_database.sh && ./scripts/setup_database.sh"
echo "   • Check Ollama:        ollama list"
echo "   • Pull new model:      ollama pull <model_name>"
echo ""

if [[ "$AUTO_START" == "true" ]]; then
    echo -e "${GREEN}🚀 Starting ReqMan...${NC}"
    cargo run --release --bin req_man
fi

echo -e "${GREEN}✅ All done! Happy requirements managing! 📋${NC}"
echo ""
