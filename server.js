const WebSocket = require('ws');

// 创建 WebSocket 服务器，监听 8080 端口
const wss = new WebSocket.Server({ host: "0.0.0.0", port: 18080 });

// 存储连接的客户端
let clients = [];

wss.on('connection', function connection(ws) {
    // 如果已经有两个客户端连接，拒绝新的连接
    if (clients.length >= 2) {
        ws.send('服务器已达到最大连接数（2个客户端）');
        ws.close();
        return;
    }

    // 将新客户端添加到数组中
    clients.push(ws);
    console.log(`新客户端连接。当前连接数: ${clients.length}`);

    // 发送欢迎消息
    ws.send(`你是客户端 #${clients.length}`);

    // 处理接收到的消息
    ws.on('message', function incoming(message) {
        console.log(`收到消息: ${message}`);
        
        // 找到另一个客户端并转发消息
        const otherClient = clients.find(client => client !== ws);
        if (otherClient && otherClient.readyState === WebSocket.OPEN) {
            otherClient.send(message.toString());
        }
    });

    // 处理客户端断开连接
    ws.on('close', function() {
        clients = clients.filter(client => client !== ws);
        console.log(`客户端断开连接。当前连接数: ${clients.length}`);
    });
});

console.log('WebSocket 服务器启动在端口 8080');