use rullst::html;
use crate::models::post::Post;

pub fn index_page(posts: Vec<Post>) -> String {
    html! {
        <html lang="en" class="dark">
            <head>
            <link rel="icon" type="image/png" href="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAACAAAAAgCAYAAABzenr0AAAKyklEQVR4nK1XaXBUVRo9977X3ekl6U7SSWhICJiwCwiDQIJDgIjAoIJhGnDUkkFEUKcUgXFjDBGZ0gFRcUOHURHUkQgiI2oQZZNtgASyErJ1EiBL70mvb7tT3TQUEZeqKb8/79at9+4571vO913gJ4xZwbE88HkAvz8PfGQdfRaCMoDgNzTSAxggsQ32Sx8xgCIPFKlgGApGiqLv/+I3P2f8j8Cjh7DFyHfYMfj5MuQ+kYtAcjMa/ECV34RzW19CM7kRAg5C6UHKCg6dIMWpYNZiKFfO+jUjV8Cj6+3QVGzGttJGFJi1gN4AnExUYeXkOOBIN8QARCWEVllGjSKj1M+jtNOI8hFfwgbyI0KFoBgGMrcYgNWK4rnF8s8TsIIjxZAbZ2LDN8ewrMyFsAyQuYNBTlCCCSsHIN/YSlmHxJEgD3gY4FSiT9GrhMUQGsSwXBqWcMwj4PRZ4Nzc0/D2IMQYIRE40tMz5Irr2etI/u4tNB2vhS7TBAgCqI4S6PoxNAyLx7IcEeFOERqTrEBPGbQqyN1hwnzgeCFylApwS4CXIdBN2yAp1c4k7Jw0LNM2YHSiUDL5zL5YrMm1JHhYQVEMGV8jO9yB+JQ+kO+KB21toSwkMhIQgOqybgW1AFpBy9R6auaDCNIwti98FxkWneJ2tLFFh1cyJREIxVOamq5YYCAW3QBV/rOC/ckDhjELlznTLpQeKZ9/EG3OwkLQoqLLIeOLIzEC4KiFpASAVg6EDqRQVSuEEaAzQEAlyjk6Cbal38F0SxfDXt2E6kbGnpsMNqhxPTuZuxaF+1+kA1KTiL5/OjNuWavcwo7RpAkMC/sHxq71yov+Oib9MJ/j//jA6sXTV68GUFTUMwQNN8BY20Ea9/mZccr0ROSk51KpokZ5pcbGDQ8r5XW5S47euih/yYQDK2T3zVYmGM18Wus6IOgEzP1Rl7kcyZ46Mal9P/9Jr/V+10NPPjxfKvNJd3Ndvd6TDyV9ZamjSXymbAtNd8+3l2A7OMyFHE3C7QA3F5AP6LHGFsIqFyAlZfellxwe2FyhsG30o7P2rn+5pqLh/YYbX1+oRhogaOH2puGMkqRyqoloTvRgNPxIiJx3bPbXTbnHW9ci7ch86L7iqFFOzLbobrhYH4z3+6R3sMy7FIXgUQQpqgNWQImKy2A831qBvjOzcY9ysYW5vCBsygLVvKW37VzZuNs3sORDtRdc2C/JK3K/wOZmIASIUVe+ARjm5WOpiacvjjj6cr//jP7L5oyE+zC1MgB7y2dw+XiFtgiEBFn/WOkr1ythISgpguK9n3yXUIUpZy+acWH5K5j56SMIXBDhCAY8fWWMe/h9Nru39uIUwoIqSihTwGSIBsEZl7Z7SSHvzOLk7ZwWBGaTvHHa26rHdj/MVOagJLsVHjr6lbI1dDsiwlUM+aoSxiyi9cw3ROtDc0CxJadfSj27xdxS6XXbLaiqTIrfumBOVk7q3qUTJZ2Qr0n00DheQwkTIRiCmGR64PanX5OmbLyXf8mswjMqoQuZTaUe0m7mZE+dVnHzBKnkTBSp8/LP8z3ge4MgD5zKyGlg4Oik6rKOLw9jRcoIMt6SDq/OHOxjbPKfHDtyoilezmx12ut37V1/fyGA4ViQ8v2p4elcwfm7BsTXyxvO9+ae0vJ8YM22fy1mzLUBIRIPleJHL25z1PMxKac9VOkhiOQgpHBDoCnoVc56U7BnxgBugkliCfIJaEyHJaH7o459AwZOyTqR4eo/ZMKMnH8fZLmPbjlXCD84jVvNCxSmDYC3QSKB6hahU6O49OgmFjDODj0pQEnYFgv9ZR1ArLsRQGmfghG6NNwSbJbLAzrUBMKkoKFFJk4B69ITiNnpYIeUv3l5v78m3Nh+hNm6u35Xb2z/prRjB8Zn/FEelb6wsrGp+vgXgPRaRuIM32l7xdE+mAoj4dgA7SXs6N4b++mrfYO/1gOuEPRiCKaKKjxICRX7ZihVdf3uGlTQfPgFkuw0iCl4HEV8o+rDMwLvspGWRg9fG9qvBBJqxZwRCzTmpJv2vL2EHOr9+ZwVa5SqG/QF2asN+aerg1TZQgirlQmJ9IQeTYvGSiG6OfQojmV8jr9bAjg5sB/pdeosxhjGz+sw99aPTAbLUlk0N7PCVZQSqpIaKuVBY/ICj0wtRnJouGbnkcVCZfPbK5Z83PHIJY9J4zsvNrS/UP8FP8HwXMIkE0w3azNNa5M/Mq0xZV7tCbgmB6JTUCGoowCPyUMwveIc4sfFwdDdbrdhxjxAn4yswdp8UlSkJFLdqZkjnuLc3VxI4Ib/Yd64D1qG9b5XXVpVwmmC9jemqu7UiuAGhVtFm7ct5HXX+ryOo/a17h8cyz27PJdifZj1CEGkJ8wlUN7tD2LRIn5miiJyBphav3sz6bnRX26dMTHrjnGNT+c4d/V9MHn2IOuCTZ2rUjTuP+/gc6WMhFv/OdO8JcPrs/OgaiXUdjBJ6L7QkKzK2OrSXlxBddwkno/Ti895FrHL3fCq8VcWVbEJ5lwjduSOwTrOwDivBu4s5/n9t87PfjX4XvU/ckK3rDC8vPsd330Yon/L9CKZMGp3Ypb/iS7jpuCxuHWCgWklX0e3moTVowymuD5+eqmNnQsPYvFklKDh1FE8AunalkyvEFgdIzAkAf4uLzwNbtAAQ7i3iSsIWvg9Ts8lR7t5MPG1pBGu74PL/IMGduzdV/aM23ahyXnaF5ArNUnuWuEmsVzjCK+jd/q2uR6X/husN8TRmng/caUZNOaxO7LWz3o/0xQFj+UA6SHFAKEA22PBid+nY6zGEB0zABtBp5ttWvXo9pYJmfzdR4TMnbdVfzun15vPhqtN/AcbZyee8CYI41JS1OWCjvVmOqW/Ko4fRBQyLT41Lk0fr1NUajVJSjPSbCnt2KGS6ntcGtWF0w+dFnsQ2A/wkwFphw7rp47CciJC0CWBC1UCJym4rhQc36yhYVZwk10/ON1S391CKfEbBLVglJlsEpmcIMlK1K8c4cBJPOQgYcGggoAgoasrKFmyjKr+SYbpB2ZUlVi3W7keOmCPFYcjgI/Pu7A8kwPXTkE/9ABmE5Er2uj4Py1V46RHgG5EClzkHHx2Gb52CV0dInydEgt6ZVn0yExxSYBPJgjKDGHlcqGHIDen+1TNIxMM0cSvKr5yDegRhqgqfmrErol9MKtDIuLGDsYv1AFNbVA+7cuxaTkyPhkxkHRnh0ndDy6EHPLlebBbVuBTGEKMQiEcoRSE48AiJ3IETE1A0ng7y9aMxKuO9uub0eVkjObCKi8ek7RkYp6OGScaibzFQ7g5KYx2OsGkk4SUd9qI7wxhqBJkIjJAJjx4NWUqFaCJZBKNzM51DKQWDDZIzAEjRKbmv8WrjrZY/rHrCBQBSrUVXPFnaL7d0u/+OjGwO6erg/ICE+7uAtFmpaK9tQ2CnqPEyzgmanmmi4sUlADKnQJjeyHjB/hQiabODvy0RcGvq4JrbbvVys3d8Zk8+4kHZhlq6zfpu7y96qEB19KCrs4OHJ9mAXUq3YrMHYXE9iCglKD80vnrgCJTd6z3R20SFMQm4l8kEDErwBUD8pJ1y1NtFRfvaSs7MU6sadLXDNXZ6FDzQblTOYbvL1zsAZgHLnpnLI6C/F/3RfQgYbVyv/JK5KIaCWWP2eK3NlJYWMjn5eXxEa9EAX8j0P8Bv4YQA2m92wMAAAAASUVORK5CYII=" />
                <meta charset="UTF-8" />
                <title>"Rullst Press Feed"</title>
                <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;500;600;700&display=swap" rel="stylesheet" />
                <style>
                    "
                    * { box-sizing: border-box; margin: 0; padding: 0; font-family: 'Outfit', sans-serif; }
                    body { background: #030712; color: #f3f4f6; min-height: 100vh; padding: 4rem 2rem; display: flex; flex-direction: column; align-items: center; }
                    .container { max-width: 800px; width: 100%; }
                    header { text-align: center; margin-bottom: 5rem; }
                    h1 { font-size: 3.5rem; font-weight: 800; background: linear-gradient(135deg, #059669, #f97316); -webkit-background-clip: text; -webkit-text-fill-color: transparent; }
                    p.sub { color: #9ca3af; font-size: 1.20rem; margin-top: 0.5rem; }
                    .post-list { display: flex; flex-direction: column; gap: 2.5rem; }
                    .card { background: rgba(17, 24, 39, 0.7); backdrop-filter: blur(12px); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 20px; padding: 2.5rem; transition: transform 0.2s, border-color 0.2s; }
                    .card:hover { transform: translateY(-3px); border-color: rgba(5, 150, 105, 0.4); }
                    .card h2 { font-size: 1.75rem; color: #ffffff; margin-bottom: 1rem; }
                    .card p { color: #9ca3af; font-size: 1rem; line-height: 1.7; margin-bottom: 1.5rem; }
                    .read-more { color: #f97316; text-decoration: none; font-weight: 600; font-size: 0.95rem; }
                    .read-more:hover { text-decoration: underline; }
                    "
                </style>
            </head>
            <body>
                <div class="container">
                    <header style="display: flex; justify-content: space-between; align-items: center;">
                        <div style="text-align: left;">
                            <h1>"RullstPress Feed"</h1>
                            <p class="sub">"Insights on hyper-performance fullstack development"</p>
                        </div>
                        <div style="display: flex; gap: 1rem; align-items: flex-start;">
                        <div style="display: flex; flex-direction: column; align-items: center; gap: 0.25rem;">
                            <a href="/nexus" style="background: rgba(5, 150, 105, 0.2); border: 1px solid rgba(5, 150, 105, 0.5); color: #10b981; padding: 0.5rem 1rem; border-radius: 0.5rem; text-decoration: none; font-weight: 600; font-size: 0.9rem;">"⚙️ Nexus CMS"</a>
                            <span style="font-size: 0.7rem; color: #94a3b8;">"(login: admin / password)"</span>
                        </div>
                            <a href="http://localhost:5555" target="_blank" style="background: rgba(249, 115, 22, 0.2); border: 1px solid rgba(249, 115, 22, 0.5); color: #f97316; padding: 0.5rem 1rem; border-radius: 0.5rem; text-decoration: none; font-weight: 600; font-size: 0.9rem;">"📊 Rullst Studio"</a>
                        </div>
                    </header>
                    <div class="post-list">
                        { rullst::html::RawHtml::new(posts.into_iter().map(|p| html! {
                            <div class="card">
                                <h2>{&p.title}</h2>
                                <p>{p.content.chars().take(100).collect::<String>()} "..."</p>
                                <a class="read-more" href={format!("/posts/{}", p.slug)}>"Read full post &rarr;"</a>
                            </div>
                        }).collect::<Vec<_>>().join("")) }
                    </div>
                </div>
            </body>
        </html>
    }
}

pub fn detail_page(post: Post) -> String {
    html! {
        <html lang="en" class="dark">
            <head>
            <link rel="icon" type="image/png" href="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAACAAAAAgCAYAAABzenr0AAAKyklEQVR4nK1XaXBUVRo9977X3ekl6U7SSWhICJiwCwiDQIJDgIjAoIJhGnDUkkFEUKcUgXFjDBGZ0gFRcUOHURHUkQgiI2oQZZNtgASyErJ1EiBL70mvb7tT3TQUEZeqKb8/79at9+4571vO913gJ4xZwbE88HkAvz8PfGQdfRaCMoDgNzTSAxggsQ32Sx8xgCIPFKlgGApGiqLv/+I3P2f8j8Cjh7DFyHfYMfj5MuQ+kYtAcjMa/ECV34RzW19CM7kRAg5C6UHKCg6dIMWpYNZiKFfO+jUjV8Cj6+3QVGzGttJGFJi1gN4AnExUYeXkOOBIN8QARCWEVllGjSKj1M+jtNOI8hFfwgbyI0KFoBgGMrcYgNWK4rnF8s8TsIIjxZAbZ2LDN8ewrMyFsAyQuYNBTlCCCSsHIN/YSlmHxJEgD3gY4FSiT9GrhMUQGsSwXBqWcMwj4PRZ4Nzc0/D2IMQYIRE40tMz5Irr2etI/u4tNB2vhS7TBAgCqI4S6PoxNAyLx7IcEeFOERqTrEBPGbQqyN1hwnzgeCFylApwS4CXIdBN2yAp1c4k7Jw0LNM2YHSiUDL5zL5YrMm1JHhYQVEMGV8jO9yB+JQ+kO+KB21toSwkMhIQgOqybgW1AFpBy9R6auaDCNIwti98FxkWneJ2tLFFh1cyJREIxVOamq5YYCAW3QBV/rOC/ckDhjELlznTLpQeKZ9/EG3OwkLQoqLLIeOLIzEC4KiFpASAVg6EDqRQVSuEEaAzQEAlyjk6Cbal38F0SxfDXt2E6kbGnpsMNqhxPTuZuxaF+1+kA1KTiL5/OjNuWavcwo7RpAkMC/sHxq71yov+Oib9MJ/j//jA6sXTV68GUFTUMwQNN8BY20Ea9/mZccr0ROSk51KpokZ5pcbGDQ8r5XW5S47euih/yYQDK2T3zVYmGM18Wus6IOgEzP1Rl7kcyZ46Mal9P/9Jr/V+10NPPjxfKvNJd3Ndvd6TDyV9ZamjSXymbAtNd8+3l2A7OMyFHE3C7QA3F5AP6LHGFsIqFyAlZfellxwe2FyhsG30o7P2rn+5pqLh/YYbX1+oRhogaOH2puGMkqRyqoloTvRgNPxIiJx3bPbXTbnHW9ci7ch86L7iqFFOzLbobrhYH4z3+6R3sMy7FIXgUQQpqgNWQImKy2A831qBvjOzcY9ysYW5vCBsygLVvKW37VzZuNs3sORDtRdc2C/JK3K/wOZmIASIUVe+ARjm5WOpiacvjjj6cr//jP7L5oyE+zC1MgB7y2dw+XiFtgiEBFn/WOkr1ythISgpguK9n3yXUIUpZy+acWH5K5j56SMIXBDhCAY8fWWMe/h9Nru39uIUwoIqSihTwGSIBsEZl7Z7SSHvzOLk7ZwWBGaTvHHa26rHdj/MVOagJLsVHjr6lbI1dDsiwlUM+aoSxiyi9cw3ROtDc0CxJadfSj27xdxS6XXbLaiqTIrfumBOVk7q3qUTJZ2Qr0n00DheQwkTIRiCmGR64PanX5OmbLyXf8mswjMqoQuZTaUe0m7mZE+dVnHzBKnkTBSp8/LP8z3ge4MgD5zKyGlg4Oik6rKOLw9jRcoIMt6SDq/OHOxjbPKfHDtyoilezmx12ut37V1/fyGA4ViQ8v2p4elcwfm7BsTXyxvO9+ae0vJ8YM22fy1mzLUBIRIPleJHL25z1PMxKac9VOkhiOQgpHBDoCnoVc56U7BnxgBugkliCfIJaEyHJaH7o459AwZOyTqR4eo/ZMKMnH8fZLmPbjlXCD84jVvNCxSmDYC3QSKB6hahU6O49OgmFjDODj0pQEnYFgv9ZR1ArLsRQGmfghG6NNwSbJbLAzrUBMKkoKFFJk4B69ITiNnpYIeUv3l5v78m3Nh+hNm6u35Xb2z/prRjB8Zn/FEelb6wsrGp+vgXgPRaRuIM32l7xdE+mAoj4dgA7SXs6N4b++mrfYO/1gOuEPRiCKaKKjxICRX7ZihVdf3uGlTQfPgFkuw0iCl4HEV8o+rDMwLvspGWRg9fG9qvBBJqxZwRCzTmpJv2vL2EHOr9+ZwVa5SqG/QF2asN+aerg1TZQgirlQmJ9IQeTYvGSiG6OfQojmV8jr9bAjg5sB/pdeosxhjGz+sw99aPTAbLUlk0N7PCVZQSqpIaKuVBY/ICj0wtRnJouGbnkcVCZfPbK5Z83PHIJY9J4zsvNrS/UP8FP8HwXMIkE0w3azNNa5M/Mq0xZV7tCbgmB6JTUCGoowCPyUMwveIc4sfFwdDdbrdhxjxAn4yswdp8UlSkJFLdqZkjnuLc3VxI4Ib/Yd64D1qG9b5XXVpVwmmC9jemqu7UiuAGhVtFm7ct5HXX+ryOo/a17h8cyz27PJdifZj1CEGkJ8wlUN7tD2LRIn5miiJyBphav3sz6bnRX26dMTHrjnGNT+c4d/V9MHn2IOuCTZ2rUjTuP+/gc6WMhFv/OdO8JcPrs/OgaiXUdjBJ6L7QkKzK2OrSXlxBddwkno/Ti895FrHL3fCq8VcWVbEJ5lwjduSOwTrOwDivBu4s5/n9t87PfjX4XvU/ckK3rDC8vPsd330Yon/L9CKZMGp3Ypb/iS7jpuCxuHWCgWklX0e3moTVowymuD5+eqmNnQsPYvFklKDh1FE8AunalkyvEFgdIzAkAf4uLzwNbtAAQ7i3iSsIWvg9Ts8lR7t5MPG1pBGu74PL/IMGduzdV/aM23ahyXnaF5ArNUnuWuEmsVzjCK+jd/q2uR6X/husN8TRmng/caUZNOaxO7LWz3o/0xQFj+UA6SHFAKEA22PBid+nY6zGEB0zABtBp5ttWvXo9pYJmfzdR4TMnbdVfzun15vPhqtN/AcbZyee8CYI41JS1OWCjvVmOqW/Ko4fRBQyLT41Lk0fr1NUajVJSjPSbCnt2KGS6ntcGtWF0w+dFnsQ2A/wkwFphw7rp47CciJC0CWBC1UCJym4rhQc36yhYVZwk10/ON1S391CKfEbBLVglJlsEpmcIMlK1K8c4cBJPOQgYcGggoAgoasrKFmyjKr+SYbpB2ZUlVi3W7keOmCPFYcjgI/Pu7A8kwPXTkE/9ABmE5Er2uj4Py1V46RHgG5EClzkHHx2Gb52CV0dInydEgt6ZVn0yExxSYBPJgjKDGHlcqGHIDen+1TNIxMM0cSvKr5yDegRhqgqfmrErol9MKtDIuLGDsYv1AFNbVA+7cuxaTkyPhkxkHRnh0ndDy6EHPLlebBbVuBTGEKMQiEcoRSE48AiJ3IETE1A0ng7y9aMxKuO9uub0eVkjObCKi8ek7RkYp6OGScaibzFQ7g5KYx2OsGkk4SUd9qI7wxhqBJkIjJAJjx4NWUqFaCJZBKNzM51DKQWDDZIzAEjRKbmv8WrjrZY/rHrCBQBSrUVXPFnaL7d0u/+OjGwO6erg/ICE+7uAtFmpaK9tQ2CnqPEyzgmanmmi4sUlADKnQJjeyHjB/hQiabODvy0RcGvq4JrbbvVys3d8Zk8+4kHZhlq6zfpu7y96qEB19KCrs4OHJ9mAXUq3YrMHYXE9iCglKD80vnrgCJTd6z3R20SFMQm4l8kEDErwBUD8pJ1y1NtFRfvaSs7MU6sadLXDNXZ6FDzQblTOYbvL1zsAZgHLnpnLI6C/F/3RfQgYbVyv/JK5KIaCWWP2eK3NlJYWMjn5eXxEa9EAX8j0P8Bv4YQA2m92wMAAAAASUVORK5CYII=" />
                <meta charset="UTF-8" />
                <title>{&post.title}</title>
                <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;500;600;700&display=swap" rel="stylesheet" />
                <style>
                    "
                    * { box-sizing: border-box; margin: 0; padding: 0; font-family: 'Outfit', sans-serif; }
                    body { background: #030712; color: #f3f4f6; min-height: 100vh; padding: 4rem 2rem; display: flex; flex-direction: column; align-items: center; }
                    .container { max-width: 700px; width: 100%; }
                    .back-link { color: #f97316; text-decoration: none; font-weight: 600; margin-bottom: 2rem; display: inline-block; }
                    h1 { font-size: 3rem; font-weight: 800; color: #ffffff; margin-bottom: 2rem; line-height: 1.2; }
                    .content { font-size: 1.15rem; color: #d1d5db; line-height: 1.8; }
                    "
                </style>
            </head>
            <body>
                <div class="container">
                    <a class="back-link" href="/">"← Back to Feed"</a>
                    <h1>{&post.title}</h1>
                    <div class="content">
                        {&post.content}
                    </div>
                </div>
            </body>
        </html>
    }
}
