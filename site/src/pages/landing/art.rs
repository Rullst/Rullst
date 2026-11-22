//! The landing's illustrations, the same in both languages: the signal map and the engine
//! orbits of the first screen, the four demo drawings and Rullst Academy's mascot.

use rullst::html;
use rullst::html::Markup;

pub fn signal_map() -> Markup {
    html! {
        <svg class="signal-map" viewBox="0 0 600 580" fill="none" aria-hidden="true" focusable="false">
            <g class="signal-routes" stroke="currentColor" stroke-width="1.5">
                <path pathLength="1" d="M0 120h40l35 35v145l30 30h80" />
                <path pathLength="1" d="M590 40h-75l-35 35v90l-60 60" />
                <path pathLength="1" d="M600 425h-65l-40 40h-65l-40 40v75" />
            </g>
            <g class="signal-nodes" fill="currentColor">
                <circle cx="0" cy="120" r="4" />
                <circle cx="590" cy="40" r="4" />
                <circle cx="600" cy="425" r="4" />
                <circle cx="390" cy="580" r="4" />
            </g>
        </svg>
    }
}

pub fn engine_orbits() -> Markup {
    html! {
        <svg class="engine-orbits" viewBox="0 0 500 340" fill="none" aria-hidden="true" focusable="false">
            <defs>
                <linearGradient id="orbit-spectrum" x1="0" y1="0" x2="1" y2="1">
                    <stop stop-color="#62a9ff" />
                    <stop offset="0.5" stop-color="#9cef8a" />
                    <stop offset="1" stop-color="#ffad72" />
                </linearGradient>
            </defs>
            <g stroke="url(#orbit-spectrum)">
                <ellipse class="orbit-track" cx="250" cy="170" rx="219" ry="94" transform="rotate(-23 250 170)" />
                <ellipse class="orbit-track" cx="250" cy="170" rx="219" ry="94" transform="rotate(23 250 170)" />
                <ellipse class="orbit-track" cx="250" cy="170" rx="97" ry="145" />
                <g class="engine-traces" stroke-width="2">
                    <path pathLength="1" d="M80 75h51l65 65h34" />
                    <path pathLength="1" d="M422 150h-54l-50 40h-45" />
                    <path pathLength="1" d="M99 278h56l53-62h34" />
                </g>
            </g>
            <g class="engine-particles" fill="#a8f4a0">
                <circle cx="72" cy="88" r="3" />
                <circle cx="430" cy="249" r="4" />
                <circle cx="170" cy="279" r="3" />
                <circle cx="301" cy="32" r="3" />
                <circle cx="327" cy="293" r="2" />
            </g>
        </svg>
    }
}

pub fn academy_crab() -> Markup {
    html! {
        <svg viewBox="0 0 300 250" fill="none" focusable="false">
            <defs>
                <linearGradient id="academy-crab-shell" x1="118" y1="80" x2="185" y2="183" gradientUnits="userSpaceOnUse">
                    <stop stop-color="#c4f995" />
                    <stop offset="1" stop-color="#63b946" />
                </linearGradient>
                <linearGradient id="academy-crab-claw" x1="40" y1="40" x2="90" y2="120" gradientUnits="userSpaceOnUse">
                    <stop stop-color="#ffbc65" />
                    <stop offset="1" stop-color="#f17e25" />
                </linearGradient>
                <linearGradient id="academy-crab-steel" x1="100" y1="170" x2="110" y2="220" gradientUnits="userSpaceOnUse">
                    <stop stop-color="#a3bdb0" />
                    <stop offset="1" stop-color="#456757" />
                </linearGradient>
            </defs>
            <ellipse class="academy-crab-shadow" cx="151" cy="230" rx="85" ry="9" fill="#030c0a" opacity=".5" />
            <g class="academy-crab-body">
                <g stroke="#294f38" stroke-width="6" stroke-linecap="round" stroke-linejoin="round">
                    <g fill="url(#academy-crab-steel)">
                        <g class="academy-crab-legs-left">
                            <path d="m106 156-35 19-14 34 19 5 15-28 30-13z" />
                            <path d="m113 172-17 28 3 21 18-1 2-21 15-21z" />
                            <path d="m96 141-43 9-19 29 18 9 22-25 32-5z" />
                        </g>
                        <g class="academy-crab-legs-right">
                            <path d="m194 156 35 19 14 34-19 5-15-28-30-13z" />
                            <path d="m187 172 17 28-3 21-18-1-2-21-15-21z" />
                            <path d="m204 141 43 9 19 29-18 9-22-25-32-5z" />
                        </g>
                    </g>
                    <g class="academy-crab-claw-left">
                        <path d="m100 126-32-5-13-29" stroke="#648b71" stroke-width="15" />
                        <circle cx="68" cy="117" r="9" fill="#bdd5b9" />
                        <path d="M58 98C30 101 15 81 22 57L42 72 45 45C69 46 85 64 79 81 76 90 69 96 58 98Z" fill="url(#academy-crab-claw)" />
                        <path d="m30 68 9 11" stroke="#ffd69b" stroke-width="4" />
                    </g>
                    <g class="academy-crab-claw-right">
                        <path d="m200 126 32-5 13-29" stroke="#648b71" stroke-width="15" />
                        <circle cx="232" cy="117" r="9" fill="#bdd5b9" />
                        <path d="M242 98c28 3 43-17 36-41l-20 15-3-27c-24 1-40 19-34 36 3 9 10 15 21 17Z" fill="url(#academy-crab-claw)" />
                        <path d="m270 68-9 11" stroke="#ffd69b" stroke-width="4" />
                    </g>
                    <path d="M122 91V65m56 26V65" stroke="#789d75" stroke-width="12" />
                    <rect x="107" y="49" width="30" height="24" rx="9" fill="#192f25" />
                    <rect x="163" y="49" width="30" height="24" rx="9" fill="#192f25" />
                    <path d="M93 98c10-19 104-19 114 0l12 51c5 28-23 41-69 41s-74-13-69-41Z" fill="url(#academy-crab-shell)" />
                    <path d="M107 100c24-9 62-9 85 0" stroke="#e0ffb8" stroke-width="5" />
                    <rect x="108" y="115" width="84" height="39" rx="17" fill="#193e2c" stroke-width="4" />
                    <path d="M141 137q9 10 18 0" stroke="#b9f583" stroke-width="3" />
                    <path d="M150 87V70" stroke="#53734b" stroke-width="4" />
                    <circle cx="150" cy="64" r="6" fill="#ff9b36" stroke-width="3" />
                    <rect x="131" y="164" width="38" height="18" rx="6" fill="#315f37" stroke-width="2" />
                    <path d="m143 169-4 4 4 4m14-8 4 4-4 4" stroke="#c7f499" stroke-width="2" />
                </g>
                <g fill="#b9f583" class="academy-crab-eyes">
                    <rect x="119" y="56" width="6" height="10" rx="3" />
                    <rect x="175" y="56" width="6" height="10" rx="3" />
                </g>
                <g fill="#a9e783">
                    <circle cx="126" cy="131" r="3" />
                    <circle cx="174" cy="131" r="3" />
                </g>
                <g fill="#e5f8c8" stroke="#4b803d" stroke-width="2">
                    <circle cx="98" cy="153" r="4" />
                    <circle cx="202" cy="153" r="4" />
                </g>
                <g stroke="#4b803d" stroke-width="1.5">
                    <path d="m96 153h4m100 0h4" />
                </g>
            </g>
        </svg>
    }
}

pub fn demo_showcase() -> Markup {
    html! {
        <svg viewBox="0 0 200 120" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="22" y="14" width="156" height="94" rx="9" />
            <path d="M22 34h156M34 24h3m7 0h3m7 0h3" />
            <rect x="36" y="48" width="55" height="43" rx="4" />
            <path d="M105 51h56m-56 13h42m-42 13h56m-56 13h29" />
        </svg>
    }
}

pub fn demo_lms() -> Markup {
    html! {
        <svg viewBox="0 0 200 120" fill="none" stroke="currentColor" stroke-width="2">
            <path d="m30 45 70-28 70 28-70 28zM55 56v29q45 32 90 0V56M170 45v42" />
            <circle cx="170" cy="93" r="5" />
            <path d="M87 45h26M100 36v18" />
        </svg>
    }
}

pub fn demo_portfolio() -> Markup {
    html! {
        <svg viewBox="0 0 200 120" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="27" y="18" width="146" height="86" rx="9" />
            <circle cx="57" cy="48" r="13" />
            <path d="M83 41h63m-63 13h42M43 79h35m-35 10h22" />
            <rect x="96" y="70" width="25" height="22" rx="3" />
            <rect x="132" y="70" width="25" height="22" rx="3" />
        </svg>
    }
}

pub fn demo_saas() -> Markup {
    html! {
        <svg viewBox="0 0 200 120" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="22" y="14" width="156" height="94" rx="9" />
            <path d="M22 34h156M64 34v74M34 24h3m7 0h3m7 0h3M34 49h18m-18 14h18m-18 14h12" />
            <rect x="78" y="47" width="35" height="20" rx="3" />
            <rect x="127" y="47" width="35" height="20" rx="3" />
            <path d="M80 93V80m18 13V76m18 17V83m18 10V74m18 19V78" />
        </svg>
    }
}
