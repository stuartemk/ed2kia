#!/bin/bash
# ed2kIA - Script de instalación systemd
# Uso: sudo bash install.sh [config_dir]

set -e

CONFIG_DIR="${1:-/etc/ed2kia}"
DATA_DIR="/opt/ed2kia"
SERVICE_NAME="ed2kia"
BIN_PATH="/opt/ed2kia/ed2kia"
USER_NAME="ed2kia"
GROUP_NAME="ed2kia"

echo "=========================================="
echo "  ed2kIA - Instalación systemd"
echo "=========================================="
echo "Config dir: $CONFIG_DIR"
echo "Data dir:   $DATA_DIR"
echo "Binary:     $BIN_PATH"
echo "=========================================="

# Verificar que se ejecuta como root
if [ "$EUID" -ne 0 ]; then
    echo "ERROR: Este script debe ejecutarse como root (sudo)"
    exit 1
fi

# Crear usuario y grupo si no existen
if ! getent group "$GROUP_NAME" > /dev/null 2>&1; then
    echo "[+] Creando grupo $GROUP_NAME..."
    groupadd --system "$GROUP_NAME"
fi

if ! getent passwd "$USER_NAME" > /dev/null 2>&1; then
    echo "[+] Creando usuario $USER_NAME..."
    useradd --system --gid "$GROUP_NAME" --home-dir "$DATA_DIR" --shell /usr/sbin/nologin "$USER_NAME"
fi

# Crear directorios
echo "[+] Creando directorios..."
mkdir -p "$CONFIG_DIR"
mkdir -p "$DATA_DIR/data"
mkdir -p "$DATA_DIR/logs"
mkdir -p "$DATA_DIR/models"
mkdir -p "$DATA_DIR/wasm_modules"

# Copiar archivo de servicio
echo "[+] Instalando servicio systemd..."
cp "$(dirname "$0")/${SERVICE_NAME}.service" /etc/systemd/system/${SERVICE_NAME}.service
chmod 644 /etc/systemd/system/${SERVICE_NAME}.service

# Copiar ejemplo de configuración si no existe
if [ ! -f "$CONFIG_DIR/ed2kia.env" ]; then
    echo "[+] Copiando configuración de ejemplo..."
    cp "$(dirname "$0")/${SERVICE_NAME}.env" "$CONFIG_DIR/ed2kia.env"
    chmod 600 "$CONFIG_DIR/ed2kia.env"
fi

# Establecer permisos
echo "[+] Estableciendo permisos..."
chown -R "$USER_NAME":"$GROUP_NAME" "$DATA_DIR"
chown -R root:"$GROUP_NAME" "$CONFIG_DIR"
chmod 750 "$CONFIG_DIR"

# Recargar systemd
echo "[+] Recargando systemd..."
systemctl daemon-reload

# Habilitar servicio
echo "[+] Habilitando servicio $SERVICE_NAME..."
systemctl enable "$SERVICE_NAME"

echo ""
echo "=========================================="
echo "  Instalación completada"
echo "=========================================="
echo ""
echo "Pasos siguientes:"
echo "  1. Editar configuración: sudo nano $CONFIG_DIR/ed2kia.env"
echo "  2. Colocar binario en: $BIN_PATH"
echo "  3. Iniciar servicio: sudo systemctl start $SERVICE_NAME"
echo "  4. Ver estado:        sudo systemctl status $SERVICE_NAME"
echo "  5. Ver logs:          sudo journalctl -u $SERVICE_NAME -f"
echo ""
echo "Comandos útiles:"
echo "  sudo systemctl restart $SERVICE_NAME    # Reiniciar"
echo "  sudo systemctl reload $SERVICE_NAME     # Recargar config"
echo "  sudo systemctl disable $SERVICE_NAME    # Deshabilitar auto-inicio"
echo "  sudo journalctl -u $SERVICE_NAME -f     # Logs en tiempo real"
echo ""
