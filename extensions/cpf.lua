local function validate_cpf(cpf)
	if not cpf:match("^%d%d%d%d%d%d%d%d%d%d%d$") then
		error("CPF deve conter exatamente 11 dígitos numéricos sem formatação")
	end

	local all_same = true
	for i = 2, 11 do
		if cpf:sub(i, i) ~= cpf:sub(1, 1) then
			all_same = false
			break
		end
	end
	if all_same then
		error("CPF inválido: dígitos repetidos")
	end

	local digits = {}
	for i = 1, 11 do
		digits[i] = tonumber(cpf:sub(i, i))
	end

	local sum1 = 0
	for i = 1, 9 do
		sum1 = sum1 + digits[i] * (11 - i)
	end
	local rest1 = sum1 % 11
	local dig1 = rest1 < 2 and 0 or (11 - rest1)
	if dig1 ~= digits[10] then
		error("Dígito verificador do CPF inválido")
	end

	local sum2 = 0
	for i = 1, 10 do
		sum2 = sum2 + digits[i] * (12 - i)
	end
	local rest2 = sum2 % 11
	local dig2 = rest2 < 2 and 0 or (11 - rest2)
	if dig2 ~= digits[11] then
		error("Dígito verificador do CPF inválido")
	end
end

return {
	prefix = "cpf_",

	pre_hook = function(ctx)
		if ctx.command == "ADD" then
			validate_cpf(ctx.value)

			local existing_key = ctx.get_key_by_value(ctx.value)
			if existing_key and existing_key ~= ctx.key then
				error("CPF já cadastrado na chave " .. existing_key)
			end
		end
	end,

	post_hook = function(ctx)
		if ctx.command == "GET" and ctx.result then
			ctx.result = ctx.result:gsub("^(%d%d%d)(%d%d%d)(%d%d%d)(%d%d)$", "%1.%2.%3-%4")
		end
	end,
}

