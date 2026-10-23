#!/usr/bin/env python3
"""Regenerates the CLDR tables behind `rullst::i18n` formatting.

Writes rullst-core/src/i18n/format/data_numbers.rs, data_dates.rs and
cldr_tests.rs. Numbers, currencies and the date-time joiners come from ICU
through Node.js `Intl` (Node 24: ICU 78 / CLDR 48); date and time patterns,
month, weekday and day-period names come from CLDR through Babel (2.17), which
keeps CLDR's U+202F where V8 writes a space. Every date output is checked
against ICU before it is written. Run `cargo fmt --all` afterwards.

Usage: python3 -I .github/generate-i18n-format-data.py
"""
import datetime
import json
import pathlib
import subprocess

from babel import Locale
from babel.dates import format_date, format_skeleton, format_time

ROOT = pathlib.Path(__file__).resolve().parent.parent
OUT = ROOT / "rullst-core/src/i18n/format"

NODE_NUMBERS = r"""
// Number, percent and currency data per locale from ICU (CLDR 48).
const L = JSON.parse(process.argv[1]);
const CUR = ['BRL', 'USD', 'EUR', 'GBP', 'JPY', 'XTS'];
function pattern(parts) {
  // Replace the digits with '#', keep everything else (affixes).
  let out = '', inNumber = false, done = false;
  for (const p of parts) {
    const num = ['integer', 'group', 'decimal', 'fraction'].includes(p.type);
    if (num) { if (!inNumber && !done) { out += '#'; inNumber = true; } }
    else { if (inNumber) { inNumber = false; done = true; } out += p.value; }
  }
  return out;
}
const res = {};
for (const l of L) {
  const nf = new Intl.NumberFormat(l, { maximumFractionDigits: 3 });
  const parts = nf.formatToParts(1234567.891);
  const group = parts.find(p => p.type === 'group').value;
  const decimal = parts.find(p => p.type === 'decimal').value;
  const minGrouping = nf.format(1234).includes(group) ? 1 : 2;
  const negative = pattern(nf.formatToParts(-1234.5));
  const pf = new Intl.NumberFormat(l, { style: 'percent' });
  const percent = [pattern(pf.formatToParts(0.25)), pattern(pf.formatToParts(-0.25))];
  const currency = [];
  for (const c of CUR) {
    const cf = new Intl.NumberFormat(l, { style: 'currency', currency: c });
    for (const v of [1234.56, -1234.56]) {
      const ps = cf.formatToParts(v);
      for (const p of ps) {
        if (p.type === 'group' && p.value !== group) throw new Error(`${l} ${c} group ${p.value}`);
        if (p.type === 'decimal' && p.value !== decimal) throw new Error(`${l} ${c} decimal`);
      }
      let pat = pattern(ps);
      if (c === 'XTS') { if (!pat.includes('XTS')) throw new Error(l + ' XTS'); pat = pat.replace('XTS', '¤'); }
      currency.push(pat);
    }
  }
  // Glue between date and time per date style (ICU uses CLDR's atTime for long/full).
  const d = new Date(Date.UTC(2026, 9, 22, 15, 45, 30));
  const glue = [];
  for (const ds of ['short', 'medium', 'long', 'full']) {
    let found = null;
    for (const ts of ['short', 'medium']) {
      const both = new Intl.DateTimeFormat(l, { dateStyle: ds, timeStyle: ts, timeZone: 'UTC' }).format(d);
      const date = new Intl.DateTimeFormat(l, { dateStyle: ds, timeZone: 'UTC' }).format(d);
      const time = new Intl.DateTimeFormat(l, { timeStyle: ts, timeZone: 'UTC' }).format(d);
      let g;
      if (both.startsWith(date) && both.endsWith(time)) g = '{1}' + both.slice(date.length, both.length - time.length) + '{0}';
      else if (both.startsWith(time) && both.endsWith(date)) g = '{0}' + both.slice(time.length, both.length - date.length) + '{1}';
      else throw new Error(`${l} ${ds} ${ts}: ${both}`);
      if (found !== null && found !== g) throw new Error(`${l} ${ds} glue differs`);
      found = g;
    }
    glue.push(found);
  }
  res[l] = { decimal, group, minGrouping, negative, percent, currency, glue };
}
console.log(JSON.stringify(res));
"""

NODE_EXPECT = r"""
// Expected outputs from ICU 78 / CLDR 48 for the Rust tests.
const [L, NUM, PCT, MINOR, CUR] = JSON.parse(process.argv[1]);
const scale = s => (s.split('.')[1] || '').length;
const out = {};
for (const l of L) {
  const r = { number: [], percent: [], currency: {} };
  for (const v of NUM) {
    const k = scale(v);
    r.number.push(new Intl.NumberFormat(l, { minimumFractionDigits: k, maximumFractionDigits: k }).format(v));
  }
  for (const v of PCT) {
    const k = Math.max(scale(v) - 2, 0);
    r.percent.push(new Intl.NumberFormat(l, { style: 'percent', minimumFractionDigits: k, maximumFractionDigits: k }).format(v));
  }
  for (const [code, digits] of CUR) {
    r.currency[code] = MINOR.map(m => {
      const neg = m.startsWith('-'); let s = neg ? m.slice(1) : m;
      s = s.padStart(digits + 1, '0');
      const amount = (neg ? '-' : '') + (digits ? s.slice(0, s.length - digits) + '.' + s.slice(s.length - digits) : s);
      return new Intl.NumberFormat(l, { style: 'currency', currency: code, minimumFractionDigits: digits, maximumFractionDigits: digits }).format(amount);
    });
  }
  out[l] = r;
}
console.log(JSON.stringify(out));
"""

TESTS_TAIL = r"""
fn locale(tag: &'static str) -> Locale {
    Locale { tag }
}

/// Splits a joined expectation, checking it has one output per sample.
fn outputs(joined: &'static str, samples: usize) -> Vec<&'static str> {
    let outputs: Vec<&str> = joined.split('|').collect();
    assert_eq!(outputs.len(), samples, "{joined}");
    outputs
}

/// Splits `;`-separated groups of outputs.
fn groups(joined: &'static str, count: usize) -> Vec<&'static str> {
    let groups: Vec<&str> = joined.split(';').collect();
    assert_eq!(groups.len(), count, "{joined}");
    groups
}

#[test]
fn numbers_percentages_and_money_match_cldr() {
    for (tag, numbers, percents, currencies) in NUMBER_CASES {
        let locale = locale(tag);
        for (value, expected) in NUMBERS.iter().zip(outputs(numbers, NUMBERS.len())) {
            let decimal: Decimal = value.parse().unwrap();
            assert_eq!(locale.format_number(decimal), expected, "{tag} {value}");
        }
        for (value, expected) in PERCENTS.iter().zip(outputs(percents, PERCENTS.len())) {
            let ratio: Decimal = value.parse().unwrap();
            assert_eq!(locale.format_percent(ratio), expected, "{tag} {value}");
        }
        for ((code, digits), joined) in CURRENCIES.iter().zip(groups(currencies, 6)) {
            let currency = Currency::new(*code, *digits).unwrap();
            for (minor, expected) in MINOR_UNITS.iter().zip(outputs(joined, MINOR_UNITS.len())) {
                assert_eq!(
                    locale.format_currency(*minor, currency),
                    expected,
                    "{tag} {code} {minor}"
                );
            }
        }
    }
    assert_eq!(NUMBER_CASES.len(), 20);
}

#[test]
fn dates_and_times_match_cldr() {
    let samples: Vec<_> = DATES
        .iter()
        .map(|&(year, month, day, hour, minute, second)| {
            NaiveDate::from_ymd_opt(year, month, day)
                .and_then(|date| date.and_hms_opt(hour, minute, second))
                .unwrap()
        })
        .collect();
    let date_styles = [
        DateStyle::Short,
        DateStyle::Medium,
        DateStyle::Long,
        DateStyle::Full,
        DateStyle::Numeric,
    ];
    let time_styles = [TimeStyle::Short, TimeStyle::Medium];
    for (tag, dates, times, with_short, with_medium) in DATE_CASES {
        let locale = locale(tag);
        for (style, joined) in date_styles.iter().zip(groups(dates, 5)) {
            for (sample, expected) in samples.iter().zip(outputs(joined, samples.len())) {
                let date = sample.date();
                assert_eq!(locale.format_date(&date, *style), expected, "{tag} {style:?} {sample}");
            }
        }
        for (style, joined) in time_styles.iter().zip(groups(times, 2)) {
            for (sample, expected) in samples.iter().zip(outputs(joined, samples.len())) {
                let time = sample.time();
                assert_eq!(locale.format_time(&time, *style), expected, "{tag} {style:?} {sample}");
            }
        }
        for (time_style, joined_styles) in time_styles.iter().zip([with_short, with_medium]) {
            for (date_style, joined) in date_styles.iter().zip(groups(joined_styles, 4)) {
                for (sample, expected) in samples.iter().zip(outputs(joined, samples.len())) {
                    assert_eq!(
                        locale.format_datetime(sample, *date_style, *time_style),
                        expected,
                        "{tag} {date_style:?} {time_style:?} {sample}"
                    );
                }
            }
        }
    }
    assert_eq!(DATE_CASES.len(), 20);
}
"""


def node(script, argument):
    run = subprocess.run(["node", "-e", script, json.dumps(argument)],
                         check=True, capture_output=True, text=True)
    return json.loads(run.stdout)


TAGS = ['en','en-GB','pt','pt-PT','es','fr','de','it','nl','ru','uk','pl','cs','ro','ar','he','ja','zh','ko','tr']
TOKENS = {'d','dd','M','MM','MMM','MMMM','y','yy','EEEE','h','H','HH','mm','ss','a'}

def rust(s):
    out = '"'
    for ch in s:
        o = ord(ch)
        if ch == '"': out += '\\"'
        elif ch == '\\': out += '\\\\'
        elif o < 0x20 or o > 0x7e: out += '\\u{%x}' % o
        else: out += ch
    return out + '"'

def convert(pattern):
    """CLDR pattern -> literal text with {token} fields."""
    out, i = '', 0
    while i < len(pattern):
        ch = pattern[i]
        if ch == "'":
            j = pattern.index("'", i + 1) if "'" in pattern[i+1:] else len(pattern)
            lit = pattern[i+1:j]
            out += "'" if lit == '' else lit
            i = j + 1
        elif ch.isascii() and ch.isalpha():
            j = i
            while j < len(pattern) and pattern[j] == ch: j += 1
            tok = pattern[i:j]
            if tok not in TOKENS: raise SystemExit(f'unknown token {tok} in {pattern!r}')
            out += '{' + tok + '}'
            i = j
        else:
            if ch in '{}#|': raise SystemExit(f'reserved char in {pattern!r}')
            out += ch; i += 1
    return out

def write_data(numbers, out_dir):
    n = ['//! CLDR 48 number, percent and currency symbols (generated from ICU 78;',
         '//! see `format/mod.rs`). `#` stands for the digits, `¤` for a currency code.',
         '',
         'use super::NumberData;',
         '',
         '/// Currencies with localized symbols, in `NumberData::currency` order.',
         'pub(super) const CURRENCIES: [&str; 5] = ["BRL", "USD", "EUR", "GBP", "JPY"];',
         '',
         f'pub(super) static NUMBERS: [NumberData; {len(TAGS)}] = [']
    for tag in TAGS:
        v = numbers[tag]
        cur = v['currency']
        for p in cur + v['percent'] + [v['negative']]:
            assert p.count('#') == 1, (tag, p)
        n.append('    NumberData {')
        n.append(f'        tag: {rust(tag)},')
        n.append(f'        decimal: {rust(v["decimal"])},')
        n.append(f'        group: {rust(v["group"])},')
        n.append(f'        min_grouping: {v["minGrouping"]},')
        n.append(f'        negative: {rust(v["negative"])},')
        n.append(f'        percent: [{rust(v["percent"][0])}, {rust(v["percent"][1])}],')
        pairs = ', '.join(f'[{rust(cur[i])}, {rust(cur[i+1])}]' for i in range(0, len(cur), 2))
        n.append(f'        currency: [{pairs}],')
        n.append('    },')
    n.append('];')
    open(f'{out_dir}/data_numbers.rs', 'w').write('\n'.join(n) + '\n')

    d = ['//! CLDR date and time patterns and names (generated from CLDR through',
         '//! Babel and ICU 78; see `format/mod.rs`). `{x}` is a CLDR field, names',
         '//! are `|`-separated, months from January and weekdays from Monday.',
         '',
         'use super::DateData;',
         '',
         f'pub(super) static DATES: [DateData; {len(TAGS)}] = [']
    for tag in TAGS:
        loc = Locale.parse(tag.replace('-', '_'))
        date = [convert(loc.date_formats[s].pattern) for s in ['short','medium','long','full']]
        date.append(convert(loc.datetime_skeletons['yMd'].pattern))
        time = [convert(loc.time_formats[s].pattern) for s in ['short','medium']]
        months_abbr = '|'.join(loc.months['format']['abbreviated'][m] for m in range(1, 13))
        months_wide = '|'.join(loc.months['format']['wide'][m] for m in range(1, 13))
        weekdays = '|'.join(loc.days['format']['wide'][w] for w in range(7))
        am = loc.day_periods['format']['abbreviated']['am']
        pm = loc.day_periods['format']['abbreviated']['pm']
        glue = numbers[tag]['glue']
        d.append('    DateData {')
        d.append(f'        tag: {rust(tag)},')
        d.append(f'        date: [{", ".join(rust(p) for p in date)}],')
        d.append(f'        time: [{", ".join(rust(p) for p in time)}],')
        d.append(f'        glue: [{", ".join(rust(p) for p in glue)}],')
        d.append(f'        months: [{rust(months_abbr)}, {rust(months_wide)}],')
        d.append(f'        weekdays: {rust(weekdays)},')
        d.append(f'        day_periods: [{rust(am)}, {rust(pm)}],')
        d.append('    },')
    d.append('];')
    open(f'{out_dir}/data_dates.rs', 'w').write('\n'.join(d) + '\n')


NUM = ['0', '7', '-7', '123', '1234', '-1234.5', '12345', '123456.7', '1234567.891', '1000000', '0.05', '-0.50',
       '123456789012345678901234567890.12']
PCT = ['0.25', '-0.25', '1', '0', '0.256', '12.3456', '1234.5', '0.0005', '-0.5']
MINOR = ['123456', '-123456', '0', '5', '-1', '123456789012']
CUR = [['BRL', 2], ['USD', 2], ['EUR', 2], ['GBP', 2], ['JPY', 0], ['XTS', 2]]
DATES = [(2026,1,5,0,5,9), (2025,2,28,12,0,0), (1999,3,1,23,59,59), (2009,4,30,9,30,5), (2026,5,17,11,59,0),
         (2024,6,9,13,1,2), (2026,7,4,1,0,0), (2026,8,31,18,45,30), (2026,9,1,12,30,0), (2026,10,22,15,45,30),
         (2026,11,11,7,7,7), (2026,12,31,22,10,50)]


def write_tests(numbers, out_file):
    glue = {t: v['glue'] for t, v in numbers.items()}
    expect = node(NODE_EXPECT, [TAGS, NUM, PCT, MINOR, CUR])
    # Node (V8) writes U+0020 where CLDR's date patterns have U+202F.
    node_dates = json.loads(subprocess.run(['node', '-e', '''
const [L, D] = JSON.parse(process.argv[1]); const o = {};
for (const l of L) { o[l] = D.map(([y,m,d,h,mi,s]) => { const t = new Date(Date.UTC(y,m-1,d,h,mi,s)); const r = {};
  for (const ds of ['short','medium','long','full']) { r['d'+ds] = new Intl.DateTimeFormat(l,{dateStyle:ds,timeZone:'UTC'}).format(t);
    for (const ts of ['short','medium']) r['dt'+ds+ts] = new Intl.DateTimeFormat(l,{dateStyle:ds,timeStyle:ts,timeZone:'UTC'}).format(t); }
  for (const ts of ['short','medium']) r['t'+ts] = new Intl.DateTimeFormat(l,{timeStyle:ts,timeZone:'UTC'}).format(t);
  r.numeric = new Intl.DateTimeFormat(l,{year:'numeric',month:'numeric',day:'numeric',timeZone:'UTC'}).format(t);
  return r; }); }
console.log(JSON.stringify(o));''', json.dumps([TAGS, DATES])], check=True, capture_output=True, text=True).stdout)
    norm = lambda s: s.replace(' ', ' ')
    lines = ['//! Expected outputs from CLDR 48 (ICU 78, through Node.js `Intl`) for every',
             '//! locale with data; dates use CLDR\'s own patterns (U+202F where browsers',
             '//! write a space) and were cross-checked with ICU. Generated: do not edit',
             '//! by hand. Each string joins one output per sample with `|`.',
             '',
             'use super::*;',
             'use crate::i18n::{Decimal, Locale};',
             'use chrono::NaiveDate;',
             '',
             f'const NUMBERS: [&str; {len(NUM)}] = [{", ".join(rust(v) for v in NUM)}];',
             f'const PERCENTS: [&str; {len(PCT)}] = [{", ".join(rust(v) for v in PCT)}];',
             f'const MINOR_UNITS: [i64; {len(MINOR)}] = [{", ".join(MINOR)}];',
             f'const CURRENCIES: [(&str, u8); {len(CUR)}] = [{", ".join(f"({rust(c)}, {d})" for c, d in CUR)}];',
             f'const DATES: [(i32, u32, u32, u32, u32, u32); {len(DATES)}] = [{", ".join(str(d) for d in DATES)}];',
             '',
             '/// `(tag, numbers, percents, currencies)`; currencies are `;`-separated',
             '/// in `CURRENCIES` order.',
             'const NUMBER_CASES: &[(&str, &str, &str, &str)] = &[']
    def joined(xs):
        for x in xs: assert '|' not in x and ';' not in x, x
        return '|'.join(xs)
    for t in TAGS:
        e = expect[t]
        cur = ';'.join(joined(e['currency'][c]) for c, _ in CUR)
        lines.append(f'    ({rust(t)}, {rust(joined(e["number"]))}, {rust(joined(e["percent"]))}, {rust(cur)}),')
    lines.append('];')
    lines += ['', '/// `(tag, dates, times, dates with the short time, dates with the medium',
              '/// time)`: `;` separates the date styles (short, medium, long, full,',
              '/// numeric; four for the joined forms) and the time styles (short, medium).',
              'const DATE_CASES: &[(&str, &str, &str, &str, &str)] = &[']
    for t in TAGS:
        bl = t.replace('-', '_')
        cols = {k: [] for k in ['short','medium','long','full','numeric','tshort','tmedium']}
        both = {(ds, ts): [] for ds in ['short','medium','long','full'] for ts in ['short','medium']}
        for i, (y, mo, d, h, mi, s) in enumerate(DATES):
            dt = datetime.datetime(y, mo, d, h, mi, s)
            n = node_dates[t][i]
            for ds in ['short','medium','long','full']:
                v = format_date(dt.date(), ds, locale=bl)
                assert norm(v) == n['d'+ds], (t, ds, v, n['d'+ds]); cols[ds].append(v)
            v = format_skeleton('yMd', dt, locale=bl)
            assert norm(v) == n['numeric'], (t, 'numeric', v, n['numeric']); cols['numeric'].append(v)
            for ts in ['short','medium']:
                v = format_time(dt.time(), ts, locale=bl)
                assert norm(v) == n['t'+ts], (t, ts, v); cols['t'+ts].append(v)
            for k, ds in enumerate(['short','medium','long','full']):
                for ts in ['short','medium']:
                    g = glue[t][k]
                    v = g.replace('{1}', cols[ds][-1]).replace('{0}', cols['t'+ts][-1])
                    assert norm(v) == n['dt'+ds+ts], (t, ds, ts, v, n['dt'+ds+ts]); both[(ds, ts)].append(v)
        dates = ';'.join(joined(cols[k]) for k in ['short','medium','long','full','numeric'])
        times = ';'.join(joined(cols[k]) for k in ['tshort','tmedium'])
        with_short = ';'.join(joined(both[(ds, 'short')]) for ds in ['short','medium','long','full'])
        with_medium = ';'.join(joined(both[(ds, 'medium')]) for ds in ['short','medium','long','full'])
        lines.append(f'    ({rust(t)}, {rust(dates)}, {rust(times)}, {rust(with_short)}, {rust(with_medium)}),')
    lines.append('];')
    lines.append(TESTS_TAIL)
    open(out_file, 'w').write('\n'.join(lines))


def main():
    numbers = node(NODE_NUMBERS, TAGS)
    write_data(numbers, OUT)
    write_tests(numbers, OUT / "cldr_tests.rs")


main()
