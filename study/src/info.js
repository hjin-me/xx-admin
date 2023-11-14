(async function _userInfo() {
    let resp = await fetch("https://pc-api.xuexi.cn/open/api/user/info", {
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
// {
//     "data": {
//         "uid": 0,
//         "nick": "UserName",
//         "avatarMediaUrl": "",
//         "gmtActive": 1548321731000,
//         "orgIds": [
//         ]
//     },
//     "message": "OK",
//     "code": 200,
//     "error": null,
//     "ok": true
// }