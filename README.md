## composer镜像代理

随着PHP的没落，提供镜像的组织越来越少。仅剩阿里、腾讯的镜像勉强可用，而且更新速度也没有保证。自己开发发布的扩展经常没法立即应用，只能通过在composer中指定临时地址的方式来处理，开发体验大降。

为了解决这个问题，用rust开发了composer的镜像代理，可通过指定扩展白名单的方式使白名单中的扩展包能快速更新。

#### 部署

这里只提供linux的部署方式，其他系统可通过编译方式案装，具体编译方式需要自行学习rust。

1. 下载 releases 里的 composer_mirror执行文件，chmod +x 设置可执行
2. 下载源码根目录下的packages.json，放到composer_mirror的存放路径
3. 在composer_mirror的存放路径下新增.env文件，用于设置环境变量（也可以不用.env，直接设置系统环境变量）
4. 需要用nginx反向代理到3000端口 （nginx完整实现了http协议，不用nginx反向代理可能会出现composer拉取内容时因为缺少http的内容实现而卡住的问题）
5. 使用supervisor之类的守护进程程序启动composer_mirror，需要注意工作目录必须时composer_mirror的位置，否则可能会出现读取不到packages.json和.env的问题

#### 环境变量设置

```shell
PORT=3000 # 服务监听端口

PACKAGE_WHITE_LIST=tiderjian/*,quansitech/*  # 需要实时更新的扩展白名单，支持 * 泛型匹配，也可以用*/*，表示所有包要实时更新
PACKAGIST_STRATEGY=2   # 扩展更新策略 1: 自己搭建存储系统, 2: 使用第三方加速地址
# 策略1 需要提供七牛云存储相关参数
DOMAIN= # 七牛云存储的自定义域名
ACCESS_KEY= # 七牛 access_key
SECRET_KEY= # 七牛 secret_key
BUCKET= # 七牛对象存储 bucket

# 策略2 需要提供加速地址 chrome有个插件叫github加速扩展，里面提供了一些用于加速的站点列表，可以复制到此处，用逗号分隔
CACHE_SITE_LIST=https://gh.api.99988866.xyz,https://gh.con.sh,https://gh.ddlc.top,https://gh2.yanqishui.work,https://ghdl.feizhuqwq.cf,https://ghproxy.com,https://ghps.cc,https://git.xfj0.cn,https://github.91chi.fun
```

##### 策略1

使用该策略需要自备七牛云存储账号，composer_mirror在检查到白名单内扩展阿里云、腾讯云镜像都查找不到时则会通过官网的包指向地址下载扩展并且上传到七牛云，返回七牛云的链接。如果阿里、腾讯、七牛已经存在该扩展，则会返回对应的扩展链接。

该策略需要准备一台能顺利访问境外网站（github）的服务器，同时还需要七牛云存储服务，有一定的额外投入。但相对来说更稳定。

##### 策略2

该策略无需任何额外的投入，只需准备一台境内服务器，将github加速插件里的加速地址设置上去即可自动检测可用的加速地址，并返回。适合小公司或者个人使用。


#### 程序流程

![流程图](https://github.com/quansitech/composer_mirror/blob/master/image.png)
