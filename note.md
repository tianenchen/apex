# 持久化

## 日志

客户端发送请求到raft,最终会定向到leader节点,leader节点负责将请求转换成raft中的日志,顺序写入到自己的Logs中

此时的日志还未持久化,真正能够确保提交是能够应用到状态机的日志,

leader发送append_entries请求给其他follower节点,此时leader节点还持有客户端的连接,

收follower节点的应答后计算follower的数量是否大于半数,这里follower的应答满足条件是十分严格的.


```
参数	解释
term	领导人的任期号
leaderId	领导人的 Id，以便于跟随者重定向请求
prevLogIndex	新的日志条目紧随之前的索引值
prevLogTerm	prevLogIndex 条目的任期号
entries[]	准备存储的日志条目（表示心跳时为空；一次性发送多个是为了提高效率）
leaderCommit	领导人已经提交的日志的索引值


返回值	解释
term	当前的任期号，用于领导人去更新自己
success	跟随者包含了匹配上 prevLogIndex 和 prevLogTerm 的日志时为真


如果 term < currentTerm 就返回 false （5.1 节）
如果日志在 prevLogIndex 位置处的日志条目的任期号和 prevLogTerm 不匹配，则返回 false （5.3 节）
如果已经存在的日志条目和新的产生冲突（索引值相同但是任期号不同），删除这一条和之后所有的 （5.3 节）
附加日志中尚未存在的任何新条目
如果 leaderCommit > commitIndex，令 commitIndex 等于 leaderCommit 和 新日志条目索引值中较小的一个
```

成功后leader将更新自己的commit_index,同时尝试直接apply到状态机中,

下次发送心跳的时候follower 收到leader 的append_entries请求时发现自己的commit_index与leader发送请求中的commited_index不一致时

就会将这条消息应用到状态机.

同时如果follower未应用到状态机而leader跪了,该follower成为了leader 则会间接接尝试commit该条消息(首先会将所有follower的状态同步然后尝试commit,因为该leader的消息必定是最新的),

所以对于客户端来说这条消息依然是提交成功的,因为最终结果都是到了状态机中

## 状态机

当leader发送append_entries请求后成功后,尝试apply日志到本地状态机,

状态机提供有快照功能

### 快照

peer定时产生快照,记录有last_index,以及last_term,快照成功则将logs中的内容清空,

节点启动时会直接加载状态机中的快照为初始状态.

状态机提供有查询接口

快照主要用于日志的压缩,方式内存损耗过大. 