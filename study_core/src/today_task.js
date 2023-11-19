(async function _todayScore() {
    let resp = await fetch("https://pc-proxy-api.xuexi.cn/delegate/score/days/listScoreProgress?sence=score&deviceType=2", {
        "headers": {
            "accept": "application/json, text/plain, */*",
        },
        "referrer": "https://pc.xuexi.cn/",
        "referrerPolicy": "strict-origin-when-cross-origin",
        "method": "GET",
        "mode": "cors",
        "credentials": "include"
    });
    return await resp.text();
})();