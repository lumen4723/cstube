let urlget = async () => {
    await fetch("http://202.30.32.104/url", {
        method: "GET",
        headers: {
            "Content-Type": "application/json",
        },
    })
    .then((res) => res.json())
    .then((json) => {
        const table = document.querySelector(".qulist");
        json.map((url) => {
            const tr = document.createElement("tr");
            
            [url.id, url.title].map((tag) => {
                const td = document.createElement("td");
                td.innerHTML = tag;
                tr.appendChild(td);
            });
            const btn = document.createElement("button");
            btn.innerHTML = "삭제"
            const td = document.createElement("td");
            td.appendChild(btn);
            tr.appendChild(td);

            table.appendChild(tr);
        })
    })
    .catch((error) => console.error('fetch failed', error));
};

urlget();
