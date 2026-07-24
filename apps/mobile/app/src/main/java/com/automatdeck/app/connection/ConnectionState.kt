package com.automatdeck.app.connection

enum class ConnectionState {
    Disconnected,
    Connecting,
    Identifying,
    Connected,
    Reconnecting,
    Failed
}
