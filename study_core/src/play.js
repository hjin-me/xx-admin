(() => {
    function getVideoTag() {

        // 视频播放按钮更新
        let video = document.querySelector("video");
        let pauseButton = document.querySelector(".prism-big-play-btn");

        return {
            video: video,
            pauseButton: pauseButton,
        };
    }

    let interval = setInterval(() => {
            let {video, pauseButton} = getVideoTag();
            if (!video) {
                console.log("等待视频加载")
                return
            }

            if (!video.muted) {
                video.muted = true
            }
            if (video.paused) {
                try {
                    Math.random() > 0.5 ? video.play() : pauseButton.click();
                } catch (e) {
                    console.warn(e)
                }
            } else {
                clearInterval(interval)
                console.log("播放成功")
            }
        }
        , 800);
})();