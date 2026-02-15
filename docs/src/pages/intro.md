---
title: Foreword
---

## The Beginning

I've been working on **Eldiron** for over three years now, starting back in **January 2022**. What began as a small hobby project—an attempt to develop an RPG creator for games similar to *Ultima 4*—quickly grew into something much bigger. *Ultima 4* was a game I loved playing as a teenager and my introduction to RPGs. Many more followed, shaping my passion for the genre.

During this time, I fell in love with the **aesthetics** of these classic games. In the **80s, 90s, and early 2000s**, developers had limited graphical capabilities, but they made up for it with **beautiful pixel art** and **unique mechanics**. I wanted to create a tool that would allow me to build games with the same level of **depth and detail** as those classics.

## Early Development

My first attempt looked something like this, around mid-2022:

![Eldiron v1](/img/docs/eldironv1.png)

I quickly realized two things: first, **working on Eldiron was incredibly fun**, and second, **things got complicated fast**. My initial approach was too simplistic—it didn’t allow for all the cool features I wanted to include.

I knew I needed to **rework the user interface**, add **scripting and node-based systems**, and create a **universal map editor** that could handle **2D, isometric, and first-person** games.

At this stage, I thought, *why not go all in?* So, I started working on a new version of Eldiron **from scratch**.

## In the Middle

I spent several months developing a new **user interface engine in Rust**. My goal was to make **Eldiron fully cross-platform** (Mac, Windows, and Linux) and **independent of third-party frameworks**. That meant **building everything from scratch**—a time-consuming task, but one I felt was **worth it in the long run**.

By mid-2023 and early 2024, the **new UI** was taking shape, but I was still figuring out the best way to implement a **flexible world editor** that could support both **2D and 3D** game development.

![Eldiron v2](/img/docs/eldironv2.png)

## On the Way to v1 – The Current State

One of the final pieces of the puzzle was choosing a **Doom-style world editor**—a decision that made creating worlds and levels **both easy and highly flexible**. Another major decision was to use **Python for scripting** while also developing a **visual, node-based scripting system** for those who prefer to work without code.

The node system is built around **Python classes (modules)**, making it possible to use the same logic in both **direct scripting and visual scripting**.

In keeping with the **retro aesthetic**, I also implemented a **software rasterizer**—if they could do it in 1990, we can certainly do it now!

With the **first public versions released in February 2025**, I’m confident that **Eldiron v1** will be ready in just a few months. I’d love for you to **join me on this journey**!

![Eldiron v3](/img/docs/screenshot.png)

---

If you’d like to support the **Eldiron** project, please consider joining my [Patreon](https://www.patreon.com/eldiron). Your support helps me continue development, commission tilesets, host databases and forums, and more.
