#!/usr/bin/env python3
"""
Generate synthetic clinical TFL RTF files for performance prototyping.
Structure is inspired by r2rtf reference output but scaled to arbitrary page counts.
"""

import argparse
import random
import os


def make_font_table():
    return (
        "{\\fonttbl"
        "{\\f0\\froman\\fcharset0 Times New Roman;}"
        "{\\f1\\fswiss\\fcharset0 Arial;}"
        "{\\f2\\fnil\\fcharset134 \\u24494?\\u-27253?\\u-30335?;}"
        "}"
    )


def make_color_table():
    return "{\\colortbl ;\\red0\\green0\\blue0;\\red255\\green255\\blue255;}"


def make_header():
    return (
        "{\\rtf1\\ansi\\ansicpg1252\\deff0\\deflang1033\\deflangfe2052"
        + make_font_table()
        + make_color_table()
        + "{\\*\\generator RTF Reader Prototype Synthetic Generator;}"
        "\\paperw15840\\paperh12240\\margl1440\\margr1440\\margt1440\\margb1440"
        "\\landscape"
    )


def make_cell(x, width_twips, border="brdrs"):
    return f"\\clbrdrt\\{border}\\brdrw15\\clbrdrl\\{border}\\brdrw15\\clbrdrb\\{border}\\brdrw15\\clbrdrr\\{border}\\brdrw15\\clvertalt\\cellx{width_twips} {x}\\cell"


def make_row(cells, widths):
    parts = ["\\trowd\\trgaph108\\trleft0\\trqc"]
    for w in widths:
        parts.append(f"\\clbrdrt\\brdrs\\brdrw15\\clbrdrl\\brdrs\\brdrw15\\clbrdrb\\brdrs\\brdrw15\\clbrdrr\\brdrs\\brdrw15\\clvertalt\\cellx{w}")
    for c, w in zip(cells, widths):
        parts.append(f"\\pard\\hyphpar0\\sb15\\sa15\\fi0\\li0\\ri0\\qc\\fs18{{\\f0 {c}}}\\cell")
    parts.append("\\intbl\\row\\pard")
    return "".join(parts)


def make_table(table_id, n_rows, page_every=10):
    widths = [2040, 1500, 1200, 3600, 2000, 2400]
    header = ["Subject ID", "Age", "Sex", "Race", "Arm", "不良事件 / AE"]
    rows = [make_row(header, widths)]

    sexes = ["M", "F"]
    races = [
        "WHITE",
        "ASIAN",
        "BLACK OR AFRICAN AMERICAN",
        "NATIVE HAWAIIAN",
    ]
    arms = ["Placebo", "Drug 10 mg", "Drug 20 mg"]
    aes = ["头痛", "恶心", "腹泻", "疲劳", "皮疹", "头晕", "呕吐", "失眠"]

    lines_in_page = 0
    for i in range(n_rows):
        cells = [
            f"01-703-{random.randint(1000, 9999):04d}",
            str(random.randint(18, 85)),
            random.choice(sexes),
            random.choice(races),
            random.choice(arms),
            random.choice(aes),
        ]
        rows.append(make_row(cells, widths))
        lines_in_page += 1
        if lines_in_page >= page_every:
            rows.append("\\page")
            lines_in_page = 0

    title = (
        f"\\pard\\qc\\fs24{{\\b Table {table_id}: Demographic and Baseline Characteristics}}\\par"
        f"\\pard\\qc\\fs18 Safety Population — Synthetic data for RTF prototype\\par\\par"
    )
    footnote = (
        "\\pard\\qc\\fs16 注：此表为合成数据，仅用于 RTF 阅读器原型性能测试。\\par"
    )
    return title + "".join(rows) + footnote


def generate(target_pages: int, out_dir: str = "test-data"):
    os.makedirs(out_dir, exist_ok=True)
    out_path = os.path.join(out_dir, f"synthetic-tfl-{target_pages}p.rtf")

    # Calibration: reference 100 pages ~= 1080 rows across 6 tables.
    total_rows = int(target_pages * 10.8)
    tables = 6 if target_pages >= 100 else max(1, target_pages // 20)
    rows_per_table = total_rows // tables
    page_every = max(1, int(rows_per_table / (target_pages / tables)))

    random.seed(42)
    parts = [make_header()]
    for i in range(1, tables + 1):
        parts.append(make_table(i, rows_per_table, page_every))
    parts.append("}")

    content = "".join(parts)
    with open(out_path, "w", encoding="utf-8") as f:
        f.write(content)

    size_mb = os.path.getsize(out_path) / 1024 / 1024
    print(f"Generated: {out_path}")
    print(f"  Target pages: {target_pages}")
    print(f"  Total rows:   {tables * rows_per_table}")
    print(f"  File size:    {size_mb:.2f} MB")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--pages", type=int, default=200, help="Target page count")
    parser.add_argument("--out-dir", default="test-data", help="Output directory")
    args = parser.parse_args()
    generate(args.pages, args.out_dir)
