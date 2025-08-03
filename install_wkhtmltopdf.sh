#!/bin/bash

# Script to install wkhtmltopdf for PDF generation in ReqMan

echo "Installing wkhtmltopdf for PDF generation..."

# Check if we're on Ubuntu/Debian
if command -v apt-get &> /dev/null; then
    echo "Detected Ubuntu/Debian system"
    
    # Update package list
    sudo apt update
    
    # Try to install wkhtmltopdf
    if sudo apt install -y wkhtmltopdf; then
        echo "wkhtmltopdf installed successfully!"
    else
        echo "wkhtmltopdf not available in default repositories."
        echo "Trying alternative installation methods..."
        
        # Try to install from snap
        if command -v snap &> /dev/null; then
            echo "Installing via snap..."
            sudo snap install wkhtmltopdf
        else
            echo "Snap not available. Please install wkhtmltopdf manually:"
            echo "1. Download from: https://wkhtmltopdf.org/downloads.html"
            echo "2. Or use: sudo apt install wkhtmltopdf"
        fi
    fi
else
    echo "Please install wkhtmltopdf manually for your system:"
    echo "Download from: https://wkhtmltopdf.org/downloads.html"
fi

# Verify installation
if command -v wkhtmltopdf &> /dev/null; then
    echo "✓ wkhtmltopdf is now available for PDF generation"
    wkhtmltopdf --version
else
    echo "✗ wkhtmltopdf installation failed or not found in PATH"
    echo "PDF generation will fall back to HTML output"
fi 