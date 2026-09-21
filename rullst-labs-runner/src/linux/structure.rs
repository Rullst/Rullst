use rullst_labs::{ExecutionFailure as Error, ExecutionLimits};
use wasmparser::Payload;

pub(super) fn validate(bytes: &[u8], limits: &ExecutionLimits) -> Result<(), Error> {
    let bad = || Error::InvalidModule;
    for payload in wasmparser::Parser::new(0).parse_all(bytes) {
        match payload.map_err(|_| bad())? {
            Payload::Version {
                encoding: wasmparser::Encoding::Module,
                ..
            }
            | Payload::End(_)
            | Payload::CustomSection(_) => (),
            Payload::TypeSection(types) if types.count() <= 128 => (),
            Payload::FunctionSection(functions) if functions.count() <= 256 => (),
            Payload::ImportSection(imports) if imports.count() == 0 => (),
            Payload::TableSection(tables) if tables.count() <= 1 => {
                for table in tables {
                    let table = table.map_err(|_| bad())?.ty;
                    if table.element_type != wasmparser::RefType::FUNCREF
                        || table.table64
                        || table.shared
                        || table.initial > 4096
                        || table.maximum.is_none_or(|max| max > 4096)
                    {
                        return Err(bad());
                    }
                }
            }
            Payload::MemorySection(memories) if memories.count() <= 1 => {
                for memory in memories {
                    let memory = memory.map_err(|_| bad())?;
                    if memory.memory64
                        || memory.shared
                        || memory.page_size_log2.is_some()
                        || memory.initial > u64::from(limits.memory_pages())
                        || memory
                            .maximum
                            .is_none_or(|max| max > u64::from(limits.memory_pages()))
                    {
                        return Err(bad());
                    }
                }
            }
            Payload::GlobalSection(globals) if globals.count() <= 16 => (),
            Payload::ExportSection(exports) if exports.count() <= 8 => (),
            Payload::ElementSection(elements) if elements.count() <= 64 => (),
            Payload::DataCountSection { count, .. } if count <= 64 => (),
            Payload::DataSection(data) if data.count() <= 64 => (),
            Payload::CodeSectionStart { count, .. } if count <= 256 => (),
            Payload::CodeSectionEntry(body) => {
                if body.range().len() > 65536 {
                    return Err(bad());
                }
                let mut count = 0u32;
                for local in body.get_locals_reader().map_err(|_| bad())? {
                    count = count
                        .checked_add(local.map_err(|_| bad())?.0)
                        .ok_or_else(bad)?;
                    if count > 1024 {
                        return Err(bad());
                    }
                }
            }
            // Start, components, tags, unknown proposals and excessive counts.
            _ => return Err(bad()),
        }
    }
    Ok(())
}
