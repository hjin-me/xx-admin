# 学习强国积分每日通报

每日爬取学习强国管理侧的数据，对组织内的学习强国积分进行排名，每日通报。

用来提醒大家好好学习。

## 部署方法

修改 config.toml.example 里面的内容

`docker run -v `pwd`:/data/config.toml hjin/xx-admin:1.0.8 -c /data/config.toml`