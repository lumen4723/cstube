let sec_to_hms = (sec) => {
    let h = Math.floor(sec / 3600);
    let m = Math.floor((sec % 3600) / 60);
    let s = sec % 60;
    return (h != 0 ? h + ':' : '') + (h != 0 || m != 0 ? m + ':' : '') + s;
}

let delurl = async (idx) => {
    const fetchPromise = fetch(`http://music.cs.oppspark.net/url/${idx}`, {
        method: "DELETE",
        headers: {
            "Content-Type": "application/json",
        },
    });

    window.alert("삭제가 완료되었습니다.");

    await fetchPromise
    .then(() => window.location.href = "/")
    .catch((error) => console.error('fetch failed', error));
};

let addurl = async (data) => {
    const fetchPromise = fetch("http://music.cs.oppspark.net/url", {
        method: "POST",
        headers: {
            "Content-Type": "application/json",
        },
        body: JSON.stringify(data),
    });

    window.alert("음원을 다운 받는중입니다.\n완료되면 자동으로 리다이렉트 됩니다.");

    await fetchPromise
    .then(() => window.location.href = "/")
    .catch((error) => console.error('fetch failed', error));
};

let geturl = async () => {
    await fetch("http://music.cs.oppspark.net/url", {
        method: "GET",
        headers: {
            "Content-Type": "application/json",
        },
    })
    .then((res) => res.json())
    .then((datas) => {
        const table = document.querySelector(".qulist");
        table.querySelectorAll('td').forEach(td => td.remove());
        
        datas.forEach((data, idx) => {
            const tr = document.createElement("tr");
            
            const td_id = document.createElement("td");
            td_id.innerHTML = idx + 1;
            tr.appendChild(td_id);
            
            const td_title = document.createElement("td");
            td_title.innerHTML = data.title;
            tr.appendChild(td_title);

            const td_duration = document.createElement("td");
            td_duration.innerHTML = sec_to_hms(data.duration);
            tr.appendChild(td_duration);

            const btn = document.createElement("button");
            btn.innerHTML = "삭제";
            btn.addEventListener('click', () => delurl(idx));

            const td_btn = document.createElement("td");
            td_btn.appendChild(btn);
            tr.appendChild(td_btn);

            table.appendChild(tr);
        })
    })
};

window.onload = () => {
    geturl();
    setInterval(geturl, 60000);
}

const form = document.querySelector('.search');

form.addEventListener('submit', (e) => {
    e.preventDefault();

    const word = document.querySelector('.input').value;

    fetch('/search?word=' + encodeURIComponent(word))
        .then(response => response.json())
        .then(datas => {
            const table = document.querySelector(".searchlist");
            table.querySelectorAll('td').forEach(td => td.remove());

            datas.forEach((data, idx) => {
                const tr = document.createElement("tr");
                
                const td_id = document.createElement("td");
                td_id.innerHTML = idx + 1;
                tr.appendChild(td_id);
                
                const td_title = document.createElement("td");
                td_title.innerHTML = data.title;
                tr.appendChild(td_title);

                const btn = document.createElement("button");
                btn.innerHTML = "추가";
                btn.addEventListener('click', () => addurl(data));

                const td_btn = document.createElement("td");
                td_btn.appendChild(btn);
                tr.appendChild(td_btn);
    
                table.appendChild(tr);
            })
        })
        .catch(error => console.error('Error:', error));
});

const playbtn = document.querySelector('.play');

playbtn.addEventListener('click', async() => {
    await fetch("http://music.cs.oppspark.net/play", {
        method: "POST",
        headers: {
            "Content-Type": "application/json",
        },
    })
    .catch(error => console.error('Error:', error));
});

const stopbtn = document.querySelector('.stop');

stopbtn.addEventListener('click', async() => {
    await fetch("http://music.cs.oppspark.net/stop", {
        method: "POST",
        headers: {
            "Content-Type": "application/json",
        },
    })
    .catch(error => console.error('Error:', error));
});

const nextbtn = document.querySelector('.next');

nextbtn.addEventListener('click', async() => {
    await fetch("http://music.cs.oppspark.net/next", {
        method: "POST",
        headers: {
            "Content-Type": "application/json",
        },
    })
    .then(() => window.location.href = "/") // 끝나는 시간을 알면 자동으로 한번 더 리다이렉트
    .catch(error => console.error('Error:', error));
});
