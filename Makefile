.PHONY: build dev test clean

.FORCE: ;

prebuild: node_modules/ static/logo.png static/logo.svg

build: prebuild
	npx vite build

dev: prebuild
	npx vite

static/logo.%: media/logo/logo.%
	@mkdir -p static
	cp $< $@

node_modules/: package-lock.json
	npm ci

uninstall:
	rm -r node_modules/

test:
	npm test

test-e2e:
	npx testcafe chrome test/e2e/ -s takeOnFails=true

typing-coverage:
	npx typescript-coverage-report

clean:
	rm -rf node_modules dist

lint:
	npx eslint --ext=js,ts,vue --max-warnings=0 src/ test/

lint-fix:
	npx eslint --ext=js,ts,vue --fix src/ test/
