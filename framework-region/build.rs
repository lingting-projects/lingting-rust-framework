use serde::Deserialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceRegion {
    iso: String,
    iso3: String,
    flag: String,
    calling_codes: Vec<String>,
    phone_prefixes: Vec<String>,
    names: SourceName,
    numeric: String,
    m49: SourceM49Code,
}

#[derive(Deserialize)]
struct SourceName {
    en: String,
    zh: String,
}

#[derive(Deserialize)]
struct SourceM49Code {
    #[serde(default)]
    region: String,
    #[serde(default)]
    subregion: String,
}

#[derive(Deserialize)]
struct SourcePhone {
    prefix: u64,
    calling: u32,
    region: String,
}

#[derive(Deserialize)]
struct SourceM49Node {
    code: String,
    name: SourceName,
    #[serde(default)]
    children: Vec<SourceM49Node>,
    #[serde(default)]
    regions: Vec<String>,
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let assets_dir = manifest_dir.join("../assets/area");
    let regions_path = assets_dir.join("regions.json");
    let phones_path = assets_dir.join("phones.json");
    let m49_path = assets_dir.join("m49.json");

    for path in [&regions_path, &phones_path, &m49_path] {
        println!("cargo:rerun-if-changed={}", path.display());
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let regions = read_json::<Vec<SourceRegion>>(&regions_path);
    let phones = read_json::<Vec<SourcePhone>>(&phones_path);
    let m49 = read_json::<SourceM49Node>(&m49_path);

    write_data_file(&out_dir, "regions", |output| {
        write_regions(output, &regions);
    });
    write_data_file(&out_dir, "phones", |output| {
        write_phones(output, &phones);
    });
    write_data_file(&out_dir, "m49", |output| {
        write_m49(output, &m49);
    });
}

fn read_json<T>(path: &PathBuf) -> T
where
    T: for<'de> Deserialize<'de>,
{
    let source = fs::read_to_string(path).unwrap();
    serde_json::from_str(&source).unwrap()
}

fn write_data_file<F>(out_dir: &Path, name: &str, write: F)
where
    F: FnOnce(&mut String),
{
    let mut output = String::new();
    write(&mut output);
    fs::write(out_dir.join(format!("{name}_data.rs")), output).unwrap();
}

fn write_regions(output: &mut String, regions: &[SourceRegion]) {
    output.push_str("static REGION_REGION_VALUES: &[Region] = &[\n");
    for region in regions {
        output.push_str("    Region { iso: ");
        write_string(output, &region.iso);
        output.push_str(", iso3: ");
        write_string(output, &region.iso3);
        output.push_str(", flag: ");
        write_string(output, &region.flag);
        output.push_str(", calling_codes: ");
        write_strings(output, &region.calling_codes);
        output.push_str(", phone_prefixes: ");
        write_strings(output, &region.phone_prefixes);
        output.push_str(", name: RegionName { en: ");
        write_string(output, &region.names.en);
        output.push_str(", zh: ");
        write_string(output, &region.names.zh);
        output.push_str(" }, numeric: ");
        write_string(output, &region.numeric);
        output.push_str(", m49: RegionM49Code { region: ");
        write_string(output, &region.m49.region);
        output.push_str(", subregion: ");
        write_string(output, &region.m49.subregion);
        output.push_str(" } },\n");
    }
    output.push_str("];\n");
    output.push_str(
        "pub static REGION_REGIONS: RegionList = RegionList::new(REGION_REGION_VALUES);\n",
    );
}

fn write_phones(output: &mut String, phones: &[SourcePhone]) {
    output.push_str("static REGION_PHONE_VALUES: &[RegionPhone] = &[\n");
    for phone in phones {
        output.push_str("    RegionPhone { prefix: ");
        output.push_str(&phone.prefix.to_string());
        output.push_str(", calling: ");
        output.push_str(&phone.calling.to_string());
        output.push_str(", region: ");
        write_string(output, &phone.region);
        output.push_str(" },\n");
    }
    output.push_str("];\n");
    output.push_str(
        "pub static REGION_PHONES: RegionPhoneList = RegionPhoneList::new(REGION_PHONE_VALUES);\n",
    );
}

fn write_m49(output: &mut String, node: &SourceM49Node) {
    output.push_str("pub static REGION_M49: RegionM49 = ");
    write_m49_node(output, node, 0);
    output.push_str(";\n");
}

fn write_m49_node(output: &mut String, node: &SourceM49Node, indent: usize) {
    output.push_str("RegionM49 { code: ");
    write_string(output, &node.code);
    output.push_str(", name: RegionName { en: ");
    write_string(output, &node.name.en);
    output.push_str(", zh: ");
    write_string(output, &node.name.zh);
    if !node.children.is_empty() {
        output.push_str(" }, children: &[\n");
        for child in &node.children {
            write_indent(output, indent + 4);
            write_m49_node(output, child, indent + 4);
            output.push_str(",\n");
        }
        write_indent(output, indent);
    } else {
        output.push_str(" }, children: &[");
    }
    output.push_str("], regions: ");
    write_strings(output, &node.regions);
    output.push_str(" }");
}

fn write_indent(output: &mut String, indent: usize) {
    output.push_str(&" ".repeat(indent));
}

fn write_strings(output: &mut String, values: &[String]) {
    output.push_str("&[");
    for value in values {
        write_string(output, value);
        output.push_str(", ");
    }
    output.push(']');
}

fn write_string(output: &mut String, value: &str) {
    output.push_str(&format!("{value:?}"));
}
