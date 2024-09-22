require('lspconfig').rust_analyzer.setup {
	settings = {
		['rust-analyzer'] = {
			cargo = {
				features = { 'bsp_rpi3' },
				target = 'aarch64-unknown-none-softfloat'
			},
			checkOnSave = {
				allTargets = false,
				extraArgs = { '--bins' }
			}
		}
	}
}
