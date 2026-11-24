# Standalone propagation source profile (V2)

`compile_generic_propagation_source(source, entry, GenericPropagationSourceLimits)`
is an explicit library entry point for a complete standalone file. It admits
postfix questions in function bodies and uses the existing declaration collector,
standard definitions, symbolic checking, finite specialization, concrete
rechecking, literals, layouts and [propagation IR/VM](generic-propagation-vm-v2.md).
Aliases, records, enums and function headers retain their strict grammar. Source
bytes are never rewritten or masked. Comments and string question marks remain
ordinary source text.

`GenericPropagationSourceLimits` contains existing whole-source pipeline limits,
a propagation profile used independently for whole symbolic checking and whole
concrete rechecking, and a whole-table operation-layout profile. Existing parser
virtual-depth/site limits remain active too. All declarations and symbolic bodies,
including unused functions and unreachable statements, are checked before a
compiled owner is returned. Each semantic stage retains its own cumulative caps.
Exhaustion returns no partial program or fallback.

The returned `CompiledGenericSource` owns the original file and optional sealed
`propagation_ir()` proof. `execute_generic_source` selects the explicit propagation
VM only when that proof exists. Compiler and runtime ranges map from original
function fragments to full-file byte/line/column coordinates. Its ordinary `ir()`
view remains useful for inspection; ordinary VM execution still refuses tables
containing questions.

`compile_generic_source`, ordinary Program compilation, CLI commands and native
image decoding remain strict. Native encoding explicitly refuses the propagation
profile, including zero-question files, until its versioned feature contract and
startup rechecking support are integrated. No emitted image claims support that
its decoder lacks. Existing generic images retain their current feature words.

Qualification covers complete-source execution, aliases/generic specialization,
original Unicode ranges, unused-body errors, runtime error mapping, stage limits,
strict headers/declarations and native-profile refusal. CLI/native integration,
explicit standard-library adapters and remaining E2/G13 work remain open. Task
Manager is unchanged.

Current propagation integration: [experimental command/native support](generic-propagation-cli-v2.md)
now enables the explicit source profile under `--experimental-generics` and native
feature word 15. Historical profiles 1/3/7 and ordinary source APIs remain strict.
