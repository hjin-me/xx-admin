(async function getTodayScore() {
    const TodayQuery = "https://pc-proxy-api.xuexi.cn/delegate/score/today/query";

    let res = await fetch(TodayQuery, {
        method: "GET",
        headers: {
            "Content-Type": "application/json",
        },
        credentials: "include",
    });
    let json = await res.json();
    console.log("今日学习积分", json?.data?.score);
    return json?.data?.score;
})();