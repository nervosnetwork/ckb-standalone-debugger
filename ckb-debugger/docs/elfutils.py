import re

from elftools.dwarf.descriptions import describe_form_class


def decode_funcname(dwarfinfo, address):
    # Go over all DIEs in the DWARF information, looking for a subprogram
    # entry with an address range that includes the given address. Note that
    # this simplifies things by disregarding subprograms that may have
    # split address ranges.
    for CU in dwarfinfo.iter_CUs():
        for DIE in CU.iter_DIEs():
            try:
                if DIE.tag == "DW_TAG_subprogram":
                    lowpc = DIE.attributes["DW_AT_low_pc"].value

                    # DWARF v4 in section 2.17 describes how to interpret the
                    # DW_AT_high_pc attribute based on the class of its form.
                    # For class 'address' it's taken as an absolute address
                    # (similarly to DW_AT_low_pc); for class 'constant', it's
                    # an offset from DW_AT_low_pc.
                    highpc_attr = DIE.attributes["DW_AT_high_pc"]
                    highpc_attr_class = describe_form_class(highpc_attr.form)
                    if highpc_attr_class == "address":
                        highpc = highpc_attr.value
                    elif highpc_attr_class == "constant":
                        highpc = lowpc + highpc_attr.value
                    else:
                        print("Error: invalid DW_AT_high_pc class:", highpc_attr_class)
                        continue

                    if lowpc <= address < highpc:
                        return DIE.attributes["DW_AT_name"].value.decode("latin-1")
            except KeyError:
                continue
    return None


def get_function_address_range_by_symtab(symtab, func):
    if not symtab:
        return None

    entries = list(
        filter(
            lambda symbol: (re.search(func, symbol.name))
            and (symbol.entry.st_info.type == "STT_FUNC"),
            symtab.iter_symbols(),
        )
    )
    if len(entries) > 1:
        print(
            "There is more than one entry matching %s, please include more chars to narrow down the search:"
            % (func)
        )
        for entry in entries:
            print(entry.name)
        return None

    if len(entries) == 0:
        print(
            "There is no entry matching %s, please check your search key word." % (func)
        )
        return None

    func_name = entries[0].name
    low_pc = entries[0].entry.st_value
    high_pc = entries[0].entry.st_value + entries[0].entry.st_size
    return (func_name, low_pc, high_pc)


def get_function_address_range_by_dwarf(dwarfinfo, func):
    if not dwarfinfo:
        return None

    # Go over all DIEs in the DWARF information, looking for a subprogram
    # entry with an address range that includes the given address. Note that
    # this simplifies things by disregarding subprograms that may have
    # split address ranges.
    for CU in dwarfinfo.iter_CUs():
        for DIE in CU.iter_DIEs():
            try:
                if DIE.tag == "DW_TAG_subprogram":
                    print("DW_AT_name: ", DIE.attributes["DW_AT_name"], func)
                    attr_name = DIE.attributes["DW_AT_name"].value.decode()
                    if not re.search(func, attr_name):
                        continue
                    # if DIE.attributes['DW_AT_name'].value.decode() != func:
                    #     continue

                    lowpc = DIE.attributes["DW_AT_low_pc"].value

                    # DWARF v4 in section 2.17 describes how to interpret the
                    # DW_AT_high_pc attribute based on the class of its form.
                    # For class 'address' it's taken as an absolute address
                    # (similarly to DW_AT_low_pc); for class 'constant', it's
                    # an offset from DW_AT_low_pc.
                    highpc_attr = DIE.attributes["DW_AT_high_pc"]
                    highpc_attr_class = describe_form_class(highpc_attr.form)
                    if highpc_attr_class == "address":
                        highpc = highpc_attr.value
                    elif highpc_attr_class == "constant":
                        highpc = lowpc + highpc_attr.value
                    else:
                        print("Error: invalid DW_AT_high_pc class:", highpc_attr_class)
                        continue

                    return (attr_name, lowpc, highpc)
            except KeyError:
                print(DIE)
                return None
    return None


def get_function_address_range(elffile, func):
    symtab = elffile.get_section_by_name(".symtab")
    dwarfinfo = elffile.get_dwarf_info()
    return get_function_address_range_by_symtab(
        symtab, func
    ) or get_function_address_range_by_dwarf(dwarfinfo, func)
