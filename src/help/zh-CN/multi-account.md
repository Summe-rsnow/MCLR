# 多账号与登录模式管理

MCLR 提供了安全便捷的多账号与皮肤管理功能，支持玩家在多个正版、第三方或离线账号之间无缝切换。

## 支持的登录类型

### 1. 微软正版账号（Microsoft Account）
- 拥有官方 Minecraft 许可证的玩家。
- 支持 OAuth2 设备代码登录（Device Flow）：启动器会提供一段 8 位字符代码，浏览器打开微软官网授权即可，启动器不触碰玩家的密码。
- 支持官方披风切换与皮肤上传。

### 2. 外部验证（Authlib-Injector）
- 支持自建 Yggdrasil 认证服务器或公开皮肤站（如 LittleSkin、Blessing Skin 等）。
- 启动时自动附加 `-javaagent:authlib-injector.jar` 注入，无需手动修改游戏客户端。

### 3. 离线模式（Offline Mode）
- 无需网络验证，支持标准 UUID 生成、旧版 UUID 生成以及自定义 UUID。
- 根据离线用户名哈希自动赋予默认的 Steve 或 Alex 皮肤模型。

## 皮肤与披风预览

进入账号详情页后，你可以实时预览当前账号的高清头部头像。如果使用微软账号，可以直接上传更换 PNG 皮肤纹理并一键切换已激活的官方披风。
