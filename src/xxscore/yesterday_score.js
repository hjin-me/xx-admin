async (d, orgGrayId) => {
    console.log("I am running", d, orgGrayId);
    let data = {
        apiCode: "ab4afc14",
        dataMap: {
            startDate: d,
            endDate: d,
            offset: 0,
            sort: "totalScore",
            pageSize: 100,
            order: "desc",
            isActivate: "",
            orgGrayId: orgGrayId,
        },
    };
    let result = await fetch("https://odrp.xuexi.cn/report/commonReport", {
        headers: {
            accept: "application/json, text/plain, */*",
            "content-type": "application/json;charset=UTF-8",
        },
        referrer: "https://study.xuexi.cn/",
        referrerPolicy: "strict-origin-when-cross-origin",
        body: JSON.stringify(data),
        method: "POST",
        mode: "cors",
        credentials: "include",
    })
        .then((resp) => resp.json())
        .then((resp) => JSON.parse(resp.data_str))
        .then((resp) => ({
            date: d,
            ...resp.dataList,
        }));
    console.log("result is", result);

    data = {
        apiCode: "955eb740",
        dataMap: {orgGrayId: orgGrayId},
    };
    result.organization_rank = await fetch("https://odrp.xuexi.cn/report/commonReport", {
        body: JSON.stringify(data),
        credentials: "include",
        headers: {
            Accept: "application/json, text/plain, */*",
            "Content-Type": "application/json;charset=utf-8",
        },
        method: "POST",
        mode: "cors",
    })
        .then((resp) => resp.json())
        .then((resp) => JSON.parse(resp.data_str))
        .then((resp) => resp.dataList.data)
    return JSON.stringify(result);
}