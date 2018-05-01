require 'shiika/program'
require 'shiika/evaluator'
require 'shiika/type'

module Shiika
  # Built-in library
  # Used by Shiika::Evaluator
  # Also used by Shiika::Program (because shiika programs implicitly
  # rely on Object class, etc.)
  module Stdlib
    include Shiika::Type
    SkObj = Shiika::Evaluator::SkObj

    def self.object_new_body_stmts
      CLASSES.first[:class_methods].first[:body]
    end

    CLASSES = [
      {
        name: "Object",
        parent: '__noparent__',
        typarams: [],
        ivars: {},
        class_methods: [
          {
            name: "new",
            ret_type_spec: TyRaw["Object"],
            param_type_specs: [],
            body: ->(env, class_obj, *args){
              sk_class_name = class_obj.sk_class_name[/Meta:(.*)/, 1] or
                raise class_obj.inspect
              sk_class = env.find_class(sk_class_name)
              sk_initializer = sk_class.sk_methods.fetch("initialize")
              ivar_values = sk_initializer.params.zip(args).map{|param, arg|
                [param.name, arg] if param.is_a?(Program::IParam)
              }.compact.to_h
              obj = SkObj.new(sk_class_name, ivar_values)
              Evaluator::Call.new(obj, "initialize", args) do |result|
                obj
              end
            }
          }
        ],
        methods: [
          {
            name: "initialize",
            param_type_specs: [],
            body: ->(){}
          }
        ]
      },
      {
        name: "Bool",
        parent: "Object",
        typarams: [],
        ivars: {},
        class_methods: [],
        methods: [
          {
            name: "initialize",
            param_type_specs: [],
            body: ->(){}
          },
        ]
      },
      {
        name: "Int",
        parent: "Object",
        typarams: [],
        ivars: {
          '@rb_val' => TyRaw['Int']
        },
        class_methods: [],
        methods: [
          {
            name: "initialize",
            param_type_specs: [],
            body: ->(){}
          },
          {
            name: "+",
            ret_type_spec: TyRaw["Int"],
            param_type_specs: [TyRaw["Int"]],
            body: ->(env, this, other){
              n = this.ivar_values['@rb_val'] + other.ivar_values['@rb_val']
              SkObj.new('Int', {'@rb_val' => n})
            }
          },
          {
            name: "abs",
            ret_type_spec: TyRaw["Int"],
            param_type_specs: [],
            body: ->(env, this){
              n = this.ivar_values['@rb_val'].abs
              SkObj.new('Int', {'@rb_val' => n})
            }
          },
          {
            name: "tmp",
            ret_type_spec: TyRaw["Int"],
            param_type_specs: [],
            body: ->(env, this){
              Evaluator::Call.new(this, "abs", []) do |result|
                n = result.ivar_values['@rb_val']
                SkObj.new('Int', {'@rb_val' => n})
              end
            }
          }
        ]
      }
    ]

    # Build Program::XX from CLASSES
    def self.sk_classes
      CLASSES.flat_map{|spec|
        sk_methods = spec[:methods].map{|x|
          params = x[:param_type_specs].map{|ty|
            Program::Param.new(name: "(no name)", type_spec: ty)
          }
          if x[:name] == "initialize"
            sk_method = Program::SkInitializer.new(
              params, x[:body], []
            )
          else
            sk_method = Program::SkMethod.new(
              name: x[:name], params: params, ret_type_spec: x[:ret_type_spec], body_stmts: x[:body], typarams: []
            )
          end
          [x[:name], sk_method]
        }.to_h
        sk_class_methods = spec[:class_methods].map{|x|
          params = x[:param_type_specs].map{|type|
            Program::Param.new(name: "(no name)", type_spec: type)
          }
          sk_method = Program::SkMethod.new(
            name: x[:name], params: params, ret_type_spec: x[:ret_type_spec], body_stmts: x[:body], typarams: []
          )
          [x[:name], sk_method]
        }.to_h
        sk_ivars = spec[:ivars].map{|name, type|
          [name, Program::SkIvar.new(name: name, type_spec: type)]
        }.to_h
        sk_class, meta_class = Program::SkClass.build(
          name: spec[:name], parent_name: spec[:parent],
          sk_ivars: sk_ivars, class_methods: sk_class_methods, sk_methods: sk_methods,
          typarams: spec[:typarams]
        )
        [[sk_class.name, sk_class],
         [meta_class.name, meta_class]]
      }.to_h
    end
  end
end
